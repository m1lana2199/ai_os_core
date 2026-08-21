use crate::{
    database, cloud_llm, context, summary, config, tools,
    events::{EventBus, SystemEvent},
    circuit_breaker::CircuitBreaker,
    metrics::{MetricsCollector, Timer}, // <--- Исправили импорт
    vector_memory::VectorMemoryStore,
};
use std::sync::OnceLock;

fn get_circuit_breaker() -> &'static CircuitBreaker {
    static CB: OnceLock<CircuitBreaker> = OnceLock::new();
    CB.get_or_init(|| CircuitBreaker::new(3, 10))
}

pub async fn run_background_worker() {
    println!("⚙️ [Enterprise Worker]: Отказоустойчивый ReAct-агент с RAG запущен.");
    let cb = get_circuit_breaker();

    loop {
        if !cb.can_execute() {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            continue;
        }

        if let Some((id, username, intent)) = database::get_next_queued_task() {
            // Исправлен вызов инкремента
            MetricsCollector::global().inc_total_tasks(); 
            let _timer = Timer::new("task_execution_pipeline");

            EventBus::global().publish(SystemEvent::TaskStarted { task_id: id.clone() });

            let cfg = config::Config::from_env();
            let base_sys_ctx = context::get_system_context();
            
            let mut agent_context = format!(
                "{}\nТы — отказоустойчивый корпоративный AI-агент. У тебя есть возможность выполнять многошаговые задачи.\n\
                Если нужно вызвать инструмент, отвечай СТРОГО в формате JSON: {{\"tool\": \"название\", \"arg\": \"аргумент\"}}.\n\
                Доступные инструменты:\n\
                - list_dir (аргумент: путь или пусто)\n\
                - read_file (аргумент: путь к файлу)\n\
                - write_file (аргумент: путь|текст)\n\
                - sys_info (аргумент: пусто)\n\
                Если задача решена, выдай итоговый ответ обычным текстом.",
                base_sys_ctx
            );

            let semantic_hits = VectorMemoryStore::global().search_semantic_context(&intent, cfg.history_limit);
            let vector_context_str = semantic_hits.join("\n");
            agent_context.push_str(&format!("\n[Semantic RAG Memory]:\n{}", vector_context_str));

            let history_vec = database::search_relevant_history(&intent, cfg.history_limit);
            let mut final_execution_result = String::new();
            let max_steps = 5;
            let mut step_failed = false;

            for step in 0..max_steps {
                MetricsCollector::global().inc_llm_queries(); // Исправлен вызов

                let llm_result = crate::llm_router::LlmRouter::global()
                   .query_with_fallback(&agent_context, &history_vec, &intent)
                   .await;
                
                let raw_response = match llm_result {
                    Ok(res) => {
                        cb.record_success();
                        res.trim().trim_matches('`').trim().to_string()
                    },
                    Err(e) => {
                        cb.record_failure();
                        step_failed = true;
                        final_execution_result = format!("❌ [LLM Error на шаге {}]: {}", step, e);
                        break;
                    }
                };

                if let Ok(tool_call) = serde_json::from_str::<tools::ToolCall>(&raw_response) {
                    println!("🤖 [Agent Step {}]: Вызов инструмента -> {} ({})", step, tool_call.tool, tool_call.arg);
                    let tool_result = tools::execute_tool(&tool_call);
                    
                    agent_context.push_str(&format!("\n[Шаг {} Результат]: Инструмент {} отработал:\n{}", step, tool_call.tool, tool_result));
                    final_execution_result = tool_result;
                } else {
                    final_execution_result = raw_response;
                    break;
                }
            }

            let summary_result = if step_failed {
                final_execution_result.clone()
            } else {
                match summary::summarize_output(&intent, &final_execution_result).await {
                    Ok(text) => text,
                    Err(_) => final_execution_result,
                }
            };

            if step_failed {
                MetricsCollector::global().inc_failed(); // Исправлен вызов
                database::update_task_status(&id, "Failed", &summary_result);
            } else {
                MetricsCollector::global().inc_completed(); // Исправлен вызов
                database::update_task_status(&id, "Completed", &summary_result);
                VectorMemoryStore::global().upsert_embedding(&summary_result);
            }

            database::save_log(&username, &intent, &summary_result);
            
            EventBus::global().publish(SystemEvent::TaskCompleted { 
                task_id: id.clone(),
                result: summary_result.clone() 
            });

            println!("✅ [Enterprise Worker]: Задача {} обработана.", id);
            MetricsCollector::global().print_report(); // Исправлен вызов
        } else {
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }
    }
}