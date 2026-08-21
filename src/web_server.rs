use axum::{
    extract::{Path, Json},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use crate::{database, metrics, config};

#[derive(Deserialize)]
struct CommandRequest {
    username: Option<String>,
    password: Option<String>,
    intent: String,
}

#[derive(Serialize)]
struct TaskSubmitResponse {
    task_id: String,
}

#[derive(Serialize)]
struct TaskStatusResponse {
    status: String,
    output: String,
}

#[derive(Deserialize)]
struct AuthRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    success: bool,
    message: String,
}

pub async fn start_web_interface() {
    let cfg = config::Config::from_env();
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/execute", post(execute_handler))
        .route("/api/task/:id", get(task_status_handler))
        .route("/api/auth", post(auth_handler))
        .route("/api/metrics", get(metrics_handler));

    let addr: SocketAddr = cfg.server_addr.parse().unwrap_or_else(|_| {
        ([127, 0, 1, 1], 8080).into()
    });

    println!("🌐 [Web Server]: Очередной шлюз задач запущен на http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler() -> Html<&'static str> {
    Html(r#"
        <!DOCTYPE html>
        <html lang="ru">
        <head>
            <meta charset="UTF-8">
            <title>AI-Native OS Core | Async Task Queue</title>
            <style>
                body { background: #0f172a; color: #f8fafc; font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; }
                .container { width: 700px; background: #1e293b; padding: 25px; border-radius: 12px; box-shadow: 0 10px 25px rgba(0,0,0,0.5); }
                .header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 15px; }
                h2 { margin: 0; color: #38bdf8; font-size: 20px; }
                .metrics-badge { background: #334155; padding: 6px 12px; border-radius: 6px; font-size: 12px; font-family: monospace; color: #38bdf8; }
                .chat-box { height: 280px; background: #0f172a; border-radius: 8px; padding: 15px; overflow-y: auto; margin-bottom: 15px; border: 1px solid #334155; font-family: monospace; white-space: pre-wrap; }
                .input-group { display: flex; flex-direction: column; gap: 10px; }
                .row { display: flex; gap: 10px; }
                input { padding: 10px 12px; border-radius: 6px; border: 1px solid #475569; background: #0f172a; color: white; font-size: 14px; }
                #username, #password { width: 140px; }
                #userInput { flex: 1; }
                button { padding: 10px 20px; border-radius: 6px; border: none; background: #0284c7; color: white; font-weight: bold; cursor: pointer; }
                button:hover { background: #0369a1; }
                .auth-row { display: flex; gap: 10px; align-items: center; margin-bottom: 10px; font-size: 13px; color: #94a3b8; }
            </style>
        </head>
        <body>
            <div class="container">
                <div class="header">
                    <h2>🚀 AI-Native OS [Async Queue]</h2>
                    <div id="metrics" class="metrics-badge">Загрузка...</div>
                </div>
                <div class="auth-row">
                    <span>Авторизация:</span>
                    <input type="text" id="username" placeholder="Логин" value="admin">
                    <input type="password" id="password" placeholder="Пароль" value="admin">
                    <button onclick="registerUser()" style="background: #334155; font-size: 12px; padding: 8px 12px;">Создать аккаунт</button>
                </div>
                <div class="chat-box" id="chat"><b>[System]:</b> Асинхронный воркер задач активен.</div>
                <div class="input-group">
                    <div class="row">
                        <input type="text" id="userInput" placeholder="Введите задачу на естественном языке..." onkeydown="if(event.key === 'Enter') sendIntent()">
                        <button onclick="sendIntent()">Отправить</button>
                    </div>
                </div>
            </div>
            <script>
                async function updateMetrics() {
                    try {
                        let res = await fetch('/api/metrics');
                        let data = await res.json();
                        document.getElementById('metrics').innerText = `Status: ${data.status} | Logs: ${data.total_logs} | DB: ${data.database_status}`;
                    } catch (e) {
                        document.getElementById('metrics').innerText = `Telemetry Error`;
                    }
                }
                setInterval(updateMetrics, 3000);
                updateMetrics();

                async function registerUser() {
                    const u = document.getElementById('username').value.trim();
                    const p = document.getElementById('password').value.trim();
                    if(!u || !p) return alert('Введите логин и пароль');
                    
                    let res = await fetch('/api/auth', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ username: u, password: p })
                    });
                    let data = await res.json();
                    alert(data.message);
                }

                async function sendIntent() {
                    const username = document.getElementById('username').value.trim() || 'anonymous';
                    const password = document.getElementById('password').value.trim();
                    const input = document.getElementById('userInput');
                    const chat = document.getElementById('chat');
                    const text = input.value.trim();
                    if(!text) return;
                    
                    chat.innerHTML += `\n\n👤 [${username}]: ${text}`;
                    input.value = '';
                    chat.scrollTop = chat.scrollHeight;

                    try {
                        let response = await fetch('/api/execute', {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ username: username, password: password, intent: text })
                        });
                        let data = await response.json();
                        
                        if (!data.task_id) {
                            chat.innerHTML += `\n❌ Ошибка: ${data.output || 'Не удалось поставить задачу'}`;
                            return;
                        }

                        let taskId = data.task_id;
                        chat.innerHTML += `\n⚙️ [Task Queued]: ID ${taskId.substring(0,8)}... Ожидание выполнения...`;
                        chat.scrollTop = chat.scrollHeight;

                        let interval = setInterval(async () => {
                            let statusRes = await fetch(`/api/task/${taskId}`);
                            let statusData = await statusRes.json();
                            
                            if (statusData.status === 'Completed' || statusData.status === 'Failed') {
                                clearInterval(interval);
                                chat.innerHTML += `\n🤖 AI OS:\n${statusData.output}`;
                                chat.scrollTop = chat.scrollHeight;
                                updateMetrics();
                            }
                        }, 1000);

                    } catch (err) {
                        chat.innerHTML += `\n❌ Ошибка сети: ${err}`;
                    }
                    chat.scrollTop = chat.scrollHeight;
                }
            </script>
        </body>
        </html>
    "#)
}

#[axum::debug_handler]
async fn auth_handler(Json(payload): Json<AuthRequest>) -> Json<AuthResponse> {
    match database::register_user(&payload.username, &payload.password) {
        Ok(true) => Json(AuthResponse { success: true, message: "✅ Пользователь успешно зарегистрирован!".into() }),
        Ok(false) => Json(AuthResponse { success: false, message: "⚠️ Пользователь уже существует.".into() }),
        Err(e) => Json(AuthResponse { success: false, message: format!("❌ Ошибка БД: {}", e) }),
    }
}

#[axum::debug_handler]
async fn metrics_handler() -> Json<metrics::SystemMetrics> {
    Json(metrics::collect_metrics())
}

#[derive(Serialize)]
enum CommandResponseOrTask {
    Task(TaskSubmitResponse),
    Error(TaskStatusResponse),
}

impl IntoResponse for CommandResponseOrTask {
    fn into_response(self) -> Response {
        match self {
            Self::Task(t) => Json(t).into_response(),
            Self::Error(e) => Json(e).into_response(),
        }
    }
}

#[axum::debug_handler]
async fn execute_handler(Json(payload): Json<CommandRequest>) -> CommandResponseOrTask {
    let username = payload.username.unwrap_or_else(|| "anonymous".to_string());
    let password = payload.password.unwrap_or_default();

    if username != "anonymous" {
        if !database::verify_user(&username, &password) {
            return CommandResponseOrTask::Error(TaskStatusResponse {
                status: "Failed".into(),
                output: "❌ [Security Error]: Неверный логин или пароль.".to_string(),
            });
        }
    }
    
    let task_id = database::create_task(&username, &payload.intent);
    CommandResponseOrTask::Task(TaskSubmitResponse { task_id })
}

#[axum::debug_handler]
async fn task_status_handler(Path(id): Path<String>) -> Json<TaskStatusResponse> {
    if let Some((status, output, _)) = database::get_task(&id) {
        Json(TaskStatusResponse { status, output })
    } else {
        Json(TaskStatusResponse { status: "NotFound".into(), output: "Задача не найдена".into() })
    }
}