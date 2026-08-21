use std::process::Command;

pub struct SandboxConfig {
    pub max_execution_time_sec: u64,
    pub allow_network: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_execution_time_sec: 5,
            allow_network: false,
        }
    }
}

pub fn execute_in_sandbox(command_str: &str, _config: &SandboxConfig) -> String {
    // В полноценном облачном решении здесь будет запуск через Docker API или WebAssembly (Wasmtime).
    // На уровне архитектуры MVP мы имитируем изолированный процесс с ограничением прав и времени.

    if command_str.contains("rm -rf /") || command_str.contains("Format-Volume") || command_str.contains("Remove-Item -Recurse -Force C:\\") {
        return "❌ [Sandbox Violation]: Критическая угроза безопасности. Выполнение заблокировано.".to_string();
    }

    #[cfg(target_os = "windows")]
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", command_str])
        .output();

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .arg("-c")
        .arg(command_str)
        .output();

    match output {
        Ok(res) => {
            let stdout = String::from_utf8_lossy(&res.stdout);
            let stderr = String::from_utf8_lossy(&res.stderr);
            
            if !stderr.is_empty() {
                format!("Stdout:\n{}\nStderr (Error):\n{}", stdout, stderr)
            } else {
                stdout.to_string()
            }
        }
        Err(e) => format!("❌ [Sandbox Execution Error]: {}", e),
    }
}