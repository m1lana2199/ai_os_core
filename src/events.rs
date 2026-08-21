use tokio::sync::broadcast;
use std::sync::OnceLock;

// Перечисление всех возможных событий в нашем ядре
#[derive(Clone, Debug)]
pub enum SystemEvent {
    TaskCreated { task_id: String, payload: String },
    TaskStarted { task_id: String },
    TaskCompleted { task_id: String, result: String },
    PluginLoaded { plugin_name: String },
    SecurityAlert { reason: String },
}

// Глобальный экземпляр шины событий (Singleton через OnceLock)
pub struct EventBus {
    sender: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    pub fn global() -> &'static EventBus {
        static INSTANCE: OnceLock<EventBus> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let (sender, _) = broadcast::channel(1000); // Буфер на 1000 событий
            EventBus { sender }
        })
    }

    pub fn publish(&self, event: SystemEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.sender.subscribe()
    }
}