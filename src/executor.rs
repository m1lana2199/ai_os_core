use std::process::Command;

const DANGEROUS_PATTERNS: &[&str] = &[
    "Remove-Item -Recurse", 
    "Format-Volume", 
    "rmdir /s", 
    "del /f /s /q C:",
    "reg delete"
];

pub fn is_dangerous(cmd: &str) -> bool {
    DANGEROUS_PATTERNS.iter().any(|&pattern| cmd.contains(pattern))
}

pub fn execute_system_command(cmd: &str) -> String {
    let formatted_cmd = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Invoke-Expression '{}'", 
        cmd.replace("'", "''")
    );

    let output = Command::new("powershell")
        .args(["/C", &formatted_cmd])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            
            if !stdout.is_empty() {
                stdout
            } else if !stderr.is_empty() {
                format!("Error: {}", stderr)
            } else {
                "Команда выполнена успешно (пустой вывод).".to_string()
            }
        }
        Err(e) => format!("❌ [Execution Error]: {}", e),
    }
}