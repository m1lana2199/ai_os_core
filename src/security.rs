pub fn is_command_safe(command: &str) -> bool {
    let cmd_lower = command.to_lowercase();
    
    // Стоп-слова и опасные паттерны для деструктивных действий
    let dangerous_patterns = [
        "rm -rf /",
        "mkfs",
        ":(){ :|:& };:", // fork bomb
        "dd if=/dev/zero",
        "shutdown",
        "reboot",
    ];

    for pattern in &dangerous_patterns {
        if cmd_lower.contains(pattern) {
            return false;
        }
    }
    true
}