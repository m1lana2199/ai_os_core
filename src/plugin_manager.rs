use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use std::thread;
use std::time::Duration;

pub struct PluginManager {
    loaded_plugins: Mutex<HashSet<String>>,
}

impl PluginManager {
    pub fn global() -> &'static PluginManager {
        static INSTANCE: once_cell::sync::Lazy<PluginManager> = once_cell::sync::Lazy::new(|| {
            PluginManager {
                loaded_plugins: Mutex::new(HashSet::new()),
            }
        });
        &INSTANCE
    }

    /// Фоновый сканер папки plugins/ (Hot-Reloading daemon)
    pub fn start_hot_reload_watcher() {
        thread::spawn(|| {
            println!("🔄 [Hot-Reloading Watcher]: Детектор плагинов запущен в фоновом потоке.");
            let plugins_dir = Path::new("plugins");

            if !plugins_dir.exists() {
                let _ = fs::create_dir_all(plugins_dir);
            }

            loop {
                let mut manager_lock = PluginManager::global().loaded_plugins.lock().unwrap();
                
                if let Ok(entries) = fs::read_dir(plugins_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                            let plugin_name = path.file_stem().unwrap().to_string_lossy().into_owned();

                            if !manager_lock.contains(&plugin_name) {
                                println!("🚀 [Hot-Reloading]: Обнаружен новый Wasm-модуль -> '{}'. Проверяем подпись...", plugin_name);
                                
                                // Тестируем валидность через наш загрузчик (Ed25519 + Wasmtime)
                                match crate::wasm_loader::execute_wasm_plugin(path.to_str().unwrap(), "") {
                                    Ok(_) => {
                                        manager_lock.insert(plugin_name.clone());
                                        println!("✅ [Hot-Reloading]: Плагин '{}' успешно зарегистрирован на лету!", plugin_name);
                                        
                                        // Публикуем событие в системную шину EventBus
                                        crate::events::EventBus::global().publish(
                                            crate::events::SystemEvent::PluginLoaded { plugin_name }
                                        );
                                    }
                                    Err(e) => {
                                        println!("❌ [Hot-Reloading Security Alert]: Отказ в подключении плагина '{}': {}", plugin_name, e);
                                    }
                                }
                            }
                        }
                    }
                }

                drop(manager_lock);
                thread::sleep(Duration::from_secs(3)); // Сканируем каждые 3 секунды
            }
        });
    }
}