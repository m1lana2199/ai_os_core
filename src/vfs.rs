use std::path::{Path, PathBuf};
use std::fs;
use std::sync::OnceLock;

pub struct VirtualFileSystem {
    safe_root: PathBuf,
}

impl VirtualFileSystem {
    pub fn global() -> &'static VirtualFileSystem {
        static VFS: OnceLock<VirtualFileSystem> = OnceLock::new();
        VFS.get_or_init(|| {
            let root = PathBuf::from("./sandbox_storage");
            if !root.exists() {
                let _ = fs::create_dir_all(&root);
                println!("📁 [VFS]: Создана изолированная корневая директория песочницы: ./sandbox_storage");
            }
            let canonical_root = fs::canonicalize(&root).unwrap_or(root);
            VirtualFileSystem { safe_root: canonical_root }
        })
    }

    /// Проверка и резолв пути с защитой от Path Traversal атаки (выхода за пределы песочницы)
    pub fn resolve_secure_path(&self, user_input_path: &str) -> Result<PathBuf, String> {
        let clean_path_str = user_input_path.trim_start_matches('/').trim_start_matches('\\');
        let target_path = if clean_path_str.is_empty() {
            self.safe_root.clone()
        } else {
            self.safe_root.join(clean_path_str)
        };

        // Нормализуем путь (если файл существует) или проверяем родительскую директорию
        let canonical_target = if target_path.exists() {
            fs::canonicalize(&target_path).map_err(|e| format!("❌ [VFS Error]: Ошибка канонизации пути: {}", e))?
        } else {
            // Если файл еще не создан (для записи), проверяем родителя
            if let Some(parent) = target_path.parent() {
                if parent.exists() {
                    let canon_parent = fs::canonicalize(parent).map_err(|e| format!("❌ [VFS Error]: Ошибка родителя: {}", e))?;
                    canon_parent.join(target_path.file_name().unwrap_or_default())
                } else {
                    return Err("❌ [VFS Security]: Родительская директория не существует.".into());
                }
            } else {
                return Err("❌ [VFS Security]: Некорректный путь.".into());
            }
        };

        // Жесткая проверка: путь должен начинаться строго с безопасного корня песочницы
        if !canonical_target.starts_with(&self.safe_root) {
            return Err(format!("🚨 [VFS Security Violation]: Попытка выхода за пределы песочницы заблокирована! Путь: '{}'", user_input_path));
        }

        Ok(canonical_target)
    }
}