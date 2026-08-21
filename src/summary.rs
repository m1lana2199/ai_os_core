use serde_json::json;

pub async fn summarize_output(user_intent: &str, raw_output: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let model_name = "llama3.2:1b";

    let truncated_output: String = raw_output.chars().take(1000).collect();

    let prompt = format!(
        "User Intent: {}\n\
         PowerShell Output/Error:\n{}\n\n\
         Task: Explain this result in Russian in 1-2 friendly, clear sentences. \
         If it's an error, explain what went wrong. If it's empty, say it's done successfully.",
        user_intent, truncated_output
    );

    let req_body = json!({
        "model": model_name,
        "prompt": prompt,
        "stream": false
    });

    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&req_body)
        .send()
        .await?;

    let res_json: serde_json::Value = res.json().await?;

    if let Some(text) = res_json.get("response").and_then(|v| v.as_str()) {
        Ok(text.trim().to_string())
    } else {
        Err("Не удалось сгенерировать саммари".into())
    }
}