mod database;
mod cloud_llm;
mod context;
mod summary;
mod config;
mod tools;
mod auth;
mod worker;
mod web_server;
mod metrics;
mod wasm_loader;
mod events;
mod circuit_breaker;
mod vector_memory;
mod plugin_manager;
mod rbac;
mod vfs;
mod mesh;
mod websocket_gateway;
mod llm_router;

fn main() -> Result<(), eframe::Error> {
    println!("🚀 Запуск AI-Native OS Desktop Environment...");

    // 1. Создаем глобальный асинхронный рантайм Tokio
    let rt = tokio::runtime::Runtime::new().expect("Не удалось запустить Tokio рантайм");

    // 2. Внутри рантайма запускаем фоновый веб-шлюз и воркер агента
    rt.spawn(async {
    let _ = database::init_db();
    plugin_manager::PluginManager::start_hot_reload_watcher();
    
    // Запуск Mesh-ноды
    let mesh_node = mesh::NodeMesh::new("Alpha-Core-Node", 8081);
    tokio::spawn(async move {
        let _ = mesh_node.start_mesh_listener().await;
    });

    // Запуск WebSocket шлюза на порту 9001
    let ws_gateway = websocket_gateway::WebSocketGateway::new(9001);
    tokio::spawn(async move {
        if let Err(e) = ws_gateway.start_gateway().await {
            println!("❌ [WebSocket Gateway Error]: {}", e);
        }
    });

    tokio::spawn(web_server::start_web_interface());
    worker::run_background_worker().await;
});

    // 3. Запускаем нативный графический интерфейс (eframe блокирует главный поток окном)
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 650.0])
            .with_title("AI-Native OS [Enterprise Core]"),
        ..Default::default()
    };

    eframe::run_native(
        "AI-Native OS",
        options,
        Box::new(|_cc| Box::new(AiOsApp::new())),
    )
}

use eframe::egui;
use std::sync::{Arc, Mutex};

struct AiOsApp {
    input_text: String,
    chat_logs: Arc<Mutex<Vec<(String, String)>>>,
    current_status: Arc<Mutex<String>>,
}

impl AiOsApp {
    fn new() -> Self {
        let logs = Arc::new(Mutex::new(vec![
            ("System".into(), "🚀 AI-Native OS Core [Local Ollama + Sandbox + Wasm Plugins] запущен и готов к работе.".into())
        ]));
        let status = Arc::new(Mutex::new("Ожидание задачи".into()));

        Self {
            input_text: String::new(),
            chat_logs: logs,
            current_status: status,
        }
    }

    // Вспомогательная функция для поиска доступных .wasm плагинов
    fn get_available_plugins(&self) -> Vec<String> {
        std::fs::read_dir("plugins")
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().extension().and_then(|s| s.to_str()) == Some("wasm")
                    })
                    .map(|e| e.file_name().to_string_lossy().replace(".wasm", ""))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl eframe::App for AiOsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.window_fill = egui::Color32::from_rgb(13, 17, 23);
        style.visuals.panel_fill = egui::Color32::from_rgb(22, 27, 34);
        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(33, 38, 45);
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(48, 54, 61);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(56, 139, 253);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(31, 111, 235);
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(56, 139, 253);
        ctx.set_style(style);

        // Боковая панель для управления плагинами (Путь Б)
        egui::SidePanel::right("plugins_panel").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading(egui::RichText::new("🧩 WASM PLUGINS").color(egui::Color32::from_rgb(56, 139, 253)).strong());
            ui.separator();
            ui.add_space(4.0);

            let plugins = self.get_available_plugins();
            if plugins.is_empty() {
                ui.label(egui::RichText::new("Папка plugins/ пуста").color(egui::Color32::from_rgb(139, 148, 158)));
            } else {
                ui.label("Доступные инструменты:");
                ui.add_space(4.0);
                for plugin in plugins {
                    ui.horizontal(|ui| {
                        ui.label(format!("• {}", plugin));
                        if ui.button("▶").clicked() {
                            // Автоматически отправляем задачу на выполнение плагина в очередь
                            let task_cmd = format!("Запусти плагин {}", plugin);
                            let task_id = database::create_task("desktop_user", &task_cmd);
                            self.chat_logs.lock().unwrap().push(("User".into(), task_cmd));
                            self.chat_logs.lock().unwrap().push(("System".into(), format!("⚙️ Плагин отправлен в очередь (ID: {})", &task_id[..8])));
                        }
                    });
                }
            }
        });

        // Центральная панель с чатом и терминалом
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("⚡ AI-NATIVE OS TERMINAL").color(egui::Color32::from_rgb(56, 139, 253)).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let status = self.current_status.lock().unwrap().clone();
                    ui.label(egui::RichText::new(format!("Status: {}", status)).color(egui::Color32::from_rgb(139, 148, 158)));
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(8.0);

            let available_height = ui.available_height() - 60.0;
            egui::ScrollArea::vertical()
                .max_height(available_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    let logs = self.chat_logs.lock().unwrap().clone();
                    for (author, text) in logs {
                        ui.group(|ui| {
                            ui.set_width(ui.available_width());
                            let color = match author.as_str() {
                                "User" => egui::Color32::from_rgb(56, 139, 253),
                                "System" => egui::Color32::from_rgb(139, 148, 158),
                                _ => egui::Color32::from_rgb(46, 160, 67),
                            };
                            ui.label(egui::RichText::new(format!("[{}]", author)).color(color).strong());
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(text).color(egui::Color32::from_rgb(201, 209, 217)));
                        });
                        ui.add_space(4.0);
                    }
                });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.input_text)
                        .hint_text("Введите задачу для локального ReAct-агента...")
                        .desired_width(ui.available_width() - 100.0)
                );

                if ui.button(egui::RichText::new("ЗАПУСТИТЬ").strong()).clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                    let text = self.input_text.trim().to_string();
                    if !text.is_empty() {
                        self.chat_logs.lock().unwrap().push(("User".into(), text.clone()));
                        
                        let task_id = database::create_task("desktop_user", &text);
                        self.chat_logs.lock().unwrap().push(("System".into(), format!("⚙️ Задача отправлена в очередь (ID: {})", &task_id[..8])));

                        self.input_text.clear();
                    }
                }
            });
        });
    }
}