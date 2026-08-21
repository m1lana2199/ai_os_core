use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshMessage {
    pub sender_id: String,
    pub action: String,
    pub payload: String,
    #[serde(default)]
    pub response: String,
}

pub struct NodeMesh {
    node_id: String,
    listen_port: u16,
}

impl NodeMesh {
    pub fn new(node_id: &str, listen_port: u16) -> Self {
        Self {
            node_id: node_id.to_string(),
            listen_port,
        }
    }

    /// Запуск серверной части ноды для приема запросов от других кластеров
    pub async fn start_mesh_listener(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("127.0.0.1:{}", self.listen_port);
        let listener = TcpListener::bind(&addr).await?;
        println!("🌐 [Node Mesh]: Нода '{}' слушает сеть на {}", self.node_id, addr);

        loop {
            let (mut socket, peer_addr) = listener.accept().await?;
            let node_id = self.node_id.clone();

            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                match socket.read(&mut buf).await {
                    Ok(n) if n > 0 => {
                        if let Ok(msg) = serde_json::from_slice::<MeshMessage>(&buf[..n]) {
                            println!("📥 [Mesh Inbound]: Получено сообщение от '{}' [Действие: {}]", msg.sender_id, msg.action);
                            
                            // Ответ кластеру (указываем все необходимые поля)
                            let response = MeshMessage {
                                sender_id: node_id,
                                action: "ACK".into(),
                                payload: "".into(), // Добавлено недостающее поле
                                response: format!("Задача обработана нодой").into(),
                            };
                            let _ = socket.write_all(&serde_json::to_vec(&response).unwrap()).await;
                        }
                    }
                    _ => {}
                }
            });
        }
    }

    /// Отправка задачи/сообщения на другую ноду кластера
    pub async fn send_to_peer(peer_addr: &str, action: &str, payload: &str, my_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut socket = TcpStream::connect(peer_addr).await?;
        let msg = MeshMessage {
            sender_id: my_id.to_string(),
            action: action.to_string(),
            payload: payload.to_string(),
            response: "".to_string(), // Добавлено недостающее поле
        };

        let data = serde_json::to_vec(&msg)?;
        socket.write_all(&data).await?;

        let mut buf = vec![0u8; 4096];
        let n = socket.read(&mut buf).await?;
        let response_str = String::from_utf8_lossy(&buf[..n]).into_owned();
        
        Ok(response_str)
    }
}