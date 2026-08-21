use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;

pub fn log_action(user_intent: &str, command: &str, response: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_line = format!(
        "[{}] INTENT: '{}' | COMMAND: '{}' | RESULT: '{}'\n",
        timestamp, user_intent, command, response.replace('\n', " ")
    );

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("ai_os.log") 
    {
        let _ = file.write_all(log_line.as_bytes());
    }
}