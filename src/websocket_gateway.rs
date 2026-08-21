use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use crate::events::{EventBus, SystemEvent};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use sysinfo::System;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct WebSocketGateway {
    port: u16,
}

impl WebSocketGateway {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn start_gateway(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("🔌 [Control Center]: Веб-интерфейс и WS шлюз доступны по адресу http://{}", addr);

        // Оборачиваем System в Arc<Mutex<>> для безопасного разделения между потоками
        let sys = Arc::new(Mutex::new(System::new_all()));

        while let Ok((mut stream, peer_addr)) = listener.accept().await {
            let sys_clone = Arc::clone(&sys);
            
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let n = match stream.read(&mut buf).await {
                    Ok(n) if n > 0 => n,
                    _ => return,
                };

                let request_str = String::from_utf8_lossy(&buf[..n]);

                // 1. Проверяем запрос на получение реальных системных метрик
                if request_str.starts_with("GET /api/metrics") {
                    let mut s = sys_clone.lock().await;
                    s.refresh_all();
                    
                    let cpu_usage = s.global_cpu_info().cpu_usage();
                    let total_ram = s.total_memory() as f32;
                    let used_ram = s.used_memory() as f32;
                    let ram_usage = if total_ram > 0.0 { (used_ram / total_ram) * 100.0 } else { 0.0 };

                    let metrics_json = format!(
                        r#"{{"cpu_usage": {:.1}, "ram_usage": {:.1}, "active_tasks": 18}}"#,
                        cpu_usage, ram_usage
                    );

                    let http_response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        metrics_json.len(),
                        metrics_json
                    );
                    let _ = stream.write_all(http_response.as_bytes()).await;
                    return;
                }

                // 2. Проверяем, это ли WebSocket подключение
                if request_str.contains("Upgrade: websocket") || request_str.contains("upgrade: websocket") {
                    if let Err(e) = handle_websocket_client(stream).await {
                        println!("❌ [WebSocket Error]: Соединение с {} разорвано: {}", peer_addr, e);
                    }
                    return;
                }

                // 3. Иначе отдаем главный HTML файл
                let html_content = if std::path::Path::new("dashboard.html").exists() {
                    std::fs::read_to_string("dashboard.html").unwrap_or_else(|_| "Error reading dashboard.html".to_string())
                } else if std::path::Path::new("src/dashboard.html").exists() {
                    std::fs::read_to_string("src/dashboard.html").unwrap_or_else(|_| "Error reading dashboard.html".to_string())
                } else {
                    "<h1>dashboard.html not found in project root!</h1>".to_string()
                };

                let http_response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    html_content.len(),
                    html_content
                );

                let _ = stream.write_all(http_response.as_bytes()).await;
            });
        }

        Ok(())
    }
}

async fn handle_websocket_client(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();

    let mut rx = EventBus::global().subscribe();

    let sender_handle = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let json_msg = match event {
                SystemEvent::TaskStarted { task_id } => {
                    format!(r#"{{"event": "TaskStarted", "task_id": "{}"}}"#, task_id)
                }
                SystemEvent::TaskCompleted { task_id, result } => {
                    format!(r#"{{"event": "TaskCompleted", "task_id": "{}", "result": "{}"}}"#, task_id, result.replace('"', "'"))
                }
                SystemEvent::PluginLoaded { plugin_name } => {
                    format!(r#"{{"event": "PluginLoaded", "plugin": "{}"}}"#, plugin_name)
                }
                SystemEvent::TaskCreated { task_id, .. } => {
                    format!(r#"{{"event": "TaskCreated", "task_id": "{}"}}"#, task_id)
                }
                SystemEvent::SecurityAlert { reason } => {
                    format!(r#"{{"event": "SecurityAlert", "reason": "{}"}}"#, reason.replace('"', "'"))
                }
            };

            if write.send(tokio_tungstenite::tungstenite::Message::Text(json_msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if msg.is_close() {
            break;
        }
    }

    sender_handle.abort();
    Ok(())
}