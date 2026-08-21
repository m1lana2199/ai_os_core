use std::env;

pub fn get_system_context() -> String {
    let mut ctx = String::from("System State:\n");
    ctx.push_str(&format!("OS: {}\n", env::consts::OS));
    ctx.push_str(&format!("Current Dir: {:?}\n", env::current_dir().unwrap_or_default()));
    // Можно добавить список файлов в текущей папке для "осознанности"
    if let Ok(paths) = std::fs::read_dir(".") {
        ctx.push_str("Files in root: ");
        for path in paths.flatten().take(5) {
            ctx.push_str(&format!("{}, ", path.file_name().to_string_lossy()));
        }
        ctx.push_str("\n");
    }
    ctx
}