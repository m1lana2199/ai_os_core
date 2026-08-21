use std::env;

#[derive(Clone)]
pub struct Config {
    pub server_addr: String,
    pub history_limit: usize,
    pub jwt_secret: String,
}

impl Config {
    pub fn from_env() -> Self {
        // Загружаем из .env если доступно, либо берем из окружения ОС
        let _ = dotenvy::dotenv();

        Self {
            server_addr: env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into()),
            history_limit: env::var("HISTORY_LIMIT")
                .unwrap_or_else(|_| "5".into())
                .parse()
                .unwrap_or(5),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "enterprise_default_fallback_secret_key_2026".into()),
        }
    }
}