use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

pub async fn query_cloud_llm(
    system_prompt: &str,
    history: &[String],
    user_intent: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
    
    // Собираем полный контекст для локальной модели (Mistral/Llama)
    let mut full_prompt = format!("System instructions:\n{}\n\n", system_prompt);
    
    if !history.is_empty() {
        full_prompt.push_str("Recent conversation history:\n");
        for h in history {
            full_prompt.push_str(&format!("- {}\n", h));
        }
        full_prompt.push('\n');
    }
    
    full_prompt.push_str(&format!("User request / Current step:\n{}", user_intent));

    let payload = OllamaRequest {
        model: "mistral".to_string(), // Или "llama3", если используется она
        prompt: full_prompt,
        stream: false,
    };

    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&payload)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(format!("❌ Ошибка Ollama API: статус {}. Убедитесь, что Ollama запущена (`ollama serve`).", res.status()).into());
    }

    let json_res: OllamaResponse = res.json().await?;
    Ok(json_res.response)
}