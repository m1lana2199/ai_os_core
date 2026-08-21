use serde_json::json;

pub async fn query_llm_core(
    system_context: &str, 
    history: &[String], 
    user_intent: &str
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let model_name = "llama3.2:1b";

    let history_str = if history.is_empty() {
        "None".to_string()
    } else {
        history.join("\n")
    };

    let formatted_prompt = format!(
        "SYSTEM CONTEXT: {}\n\
         RECENT HISTORY:\n{}\n\n\
         You are a strict PowerShell command generator. Output ONLY a valid PowerShell command, nothing else.\n\n\
         Examples:\n\
         User: покажи файлы\n\
         Command: Get-ChildItem\n\
         User: создай папку test_folder\n\
         Command: New-Item -ItemType Directory -Name 'test_folder'\n\
         User: узнай свободное место\n\
         Command: Get-PSDrive C\n\
         User: покажи процессы\n\
         Command: Get-Process\n\
         User: удали папку test_folder\n\
         Command: Remove-Item -Path 'test_folder' -Recurse -Force\n\
         User: выключи блокнот\n\
         Command: Stop-Process -Name 'notepad' -Force\n\
         User: открой блокнот\n\
         Command: Start-Process 'notepad'\n\n\
         STRICT RULES:\n\
         - If user wants to DELETE/REMOVE something, use Remove-Item, NEVER Get-ChildItem.\n\
         - No markdown, no explanations, no quotes around the command.\n\n\
         User Request: {}\n\
         Command:",
        system_context, history_str, user_intent
    );

    let req_body = json!({
        "model": model_name,
        "prompt": formatted_prompt,
        "stream": false
    });

    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&req_body)
        .send()
        .await?;

    let res_json: serde_json::Value = res.json().await?;

    if let Some(text) = res_json.get("response").and_then(|v| v.as_str()) {
        Ok(text.to_string())
    } else {
        Err("Ошибка получения ответа от Ollama".into())
    }
}