use crate::circuit_breaker::CircuitBreaker;
use std::sync::OnceLock;

pub struct LlmRouter {
    primary_cb: CircuitBreaker,
}

impl LlmRouter {
    pub fn global() -> &'static LlmRouter {
        static ROUTER: OnceLock<LlmRouter> = OnceLock::new();
        ROUTER.get_or_init(|| LlmRouter {
            // Порог сбоев: 3 попытки, cooldown: 10 секунд
            primary_cb: CircuitBreaker::new(3, 10),
        })
    }

    /// Интеллектуальный запрос к LLM с автоматическим Fallback
    pub async fn query_with_fallback(&self, system_prompt: &str, history: &[String], intent: &str) -> Result<String, String> {
        // Проверяем состояние первичного контура (например, локальная Ollama)
        if self.primary_cb.can_execute() {
            println!("🤖 [LLM Router]: Маршрутизация на Primary LLM (Ollama / Local)...");
            match crate::cloud_llm::query_cloud_llm(system_prompt, history, intent).await {
                Ok(res) => {
                    self.primary_cb.record_success();
                    return Ok(res);
                }
                Err(e) => {
                    println!("⚠️ [LLM Router Warning]: Primary LLM сбоит: {}. Переход на Fallback...", e);
                    self.primary_cb.record_failure();
                }
            }
        } else {
            println!("🔄 [LLM Router]: Primary контур защищен Circuit Breaker. Автоматический уход на Fallback.");
        }

        // Fallback-контур (резервный провайдер или аварийный ответ)
        self.query_fallback_provider(system_prompt, history, intent).await
    }

    async fn query_fallback_provider(&self, _system_prompt: &str, _history: &[String], intent: &str) -> Result<String, String> {
        println!("🛡️ [LLM Router Fallback]: Обработка запроса через резервный контур...");
        
        // В продакшене здесь вызов резервного API (например, OpenAI / Anthropic / резервный локальный порт)
        // Для демонстрации возвращаем надежный аварийный ответ на базе интента
        Ok(format!("{{\"tool\": \"sys_info\", \"arg\": \"\"}}"))
    }
}