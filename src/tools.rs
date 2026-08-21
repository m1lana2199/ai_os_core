use serde::{Deserialize, Serialize};
use std::fs;
use crate::rbac::{RbacSystem, Role};
use crate::vfs::VirtualFileSystem;

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolCall {
    pub tool: String,
    pub arg: String,
}

// Защищенный запуск инструмента с проверкой прав RBAC
pub fn execute_tool_with_role(tool_call: &ToolCall, role: Role) -> String {
    if !RbacSystem::global().check_permission(role, &tool_call.tool) {
        let err_msg = format!("❌ [RBAC Security Violation]: Роль {:?} не имеет прав на вызов инструмента '{}'!", role, tool_call.tool);
        println!("{}", err_msg);
        return err_msg;
    }

    execute_tool(tool_call)
}

// Базовый диспетчер инструментов с интегрированной VFS песочницей
pub fn execute_tool(tool_call: &ToolCall) -> String {
    match tool_call.tool.as_str() {
        "list_dir" => {
            let path_arg = if tool_call.arg.is_empty() { "." } else { &tool_call.arg };
            match VirtualFileSystem::global().resolve_secure_path(path_arg) {
                Ok(secure_path) => {
                    match fs::read_dir(&secure_path) {
                        Ok(entries) => {
                            let mut list = String::new();
                            for entry in entries.flatten() {
                                list.push_str(&format!("{}\n", entry.file_name().to_string_lossy()));
                            }
                            if list.is_empty() { "📂 Директория пуста".into() } else { list }
                        }
                        Err(e) => format!("❌ Ошибка чтения директории: {}", e),
                    }
                }
                Err(err_msg) => err_msg,
            }
        }
        "read_file" => {
            if tool_call.arg.is_empty() {
                return "❌ Ошибка: не указан путь к файлу".into();
            }
            match VirtualFileSystem::global().resolve_secure_path(&tool_call.arg) {
                Ok(secure_path) => match fs::read_to_string(&secure_path) {
                    Ok(content) => content,
                    Err(e) => format!("❌ Ошибка чтения файла: {}", e),
                },
                Err(err_msg) => err_msg,
            }
        }
        "write_file" => {
            let parts: Vec<&str> = tool_call.arg.splitn(2, '|').collect();
            if parts.len() < 2 {
                return "❌ Ошибка формата. Используйте: путь|текст".into();
            }
            match VirtualFileSystem::global().resolve_secure_path(parts[0]) {
                Ok(secure_path) => match fs::write(&secure_path, parts[1]) {
                    Ok(_) => format!("✅ Файл успешно записан в защищенную песочницу."),
                    Err(e) => format!("❌ Ошибка записи файла: {}", e),
                },
                Err(err_msg) => err_msg,
            }
        }
        "sys_info" => {
            let os = std::env::consts::OS;
            let arch = std::env::consts::ARCH;
            format!("OS: {}, Arch: {}, CPU cores: {}", os, arch, num_cpus::get())
        }
        "wasm_exec" => {
            let plugin_path = format!("plugins/{}.wasm", tool_call.arg);
            match crate::wasm_loader::execute_wasm_plugin(&plugin_path, "") {
                Ok(res) => res,
                Err(e) => format!("❌ Ошибка выполнения Wasm плагина: {}", e),
            }
        }
        _ => format!("❌ Неизвестный инструмент: {}", tool_call.tool),
    }
}