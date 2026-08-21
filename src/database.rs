use rusqlite::{Connection, Result};
use uuid::Uuid;

pub fn init_db() -> Result<Connection> {
    let conn = Connection::open("ai_os.db")?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            intent TEXT NOT NULL,
            output TEXT NOT NULL,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS logs_fts USING fts5(intent, output)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            intent TEXT NOT NULL,
            status TEXT NOT NULL,
            output TEXT
        )",
        [],
    )?;

    Ok(conn)
}

pub fn register_user(username: &str, password: &str) -> Result<bool> {
    let conn = init_db()?;
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM users WHERE username = ?1")?;
    let count: i64 = stmt.query_row([username], |row| row.get(0))?;
    
    if count > 0 {
        return Ok(false);
    }

    let password_hash = format!("{:x}", simple_hash(password)); 
    conn.execute(
        "INSERT INTO users (username, password_hash) VALUES (?1, ?2)",
        [username, &password_hash],
    )?;
    Ok(true)
}

pub fn verify_user(username: &str, password: &str) -> bool {
    let conn = match init_db() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let password_hash = format!("{:x}", simple_hash(password));
    let mut stmt = match conn.prepare("SELECT COUNT(*) FROM users WHERE username = ?1 AND password_hash = ?2") {
        Ok(s) => s,
        Err(_) => return false,
    };
    let count: i64 = stmt.query_row([username, &password_hash], |row| row.get(0)).unwrap_or(0);
    count > 0
}

fn simple_hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

pub fn save_log(username: &str, intent: &str, output: &str) {
    if let Ok(conn) = init_db() {
        let _ = conn.execute(
            "INSERT INTO logs (username, intent, output) VALUES (?1, ?2, ?3)",
            [username, intent, output],
        );
        let _ = conn.execute(
            "INSERT INTO logs_fts (intent, output) VALUES (?1, ?2)",
            [intent, output],
        );
    }
}

pub fn get_recent_history(limit: usize) -> Vec<String> {
    let conn = match init_db() {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut stmt = match conn.prepare("SELECT intent, output FROM logs ORDER BY id DESC LIMIT ?1") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows = stmt.query_map([limit], |row| {
        let intent: String = row.get(0)?;
        let output: String = row.get(1)?;
        Ok(format!("User: {}\nAI: {}", intent, output))
    });

    let mut history = Vec::new();
    if let Ok(mapped) = rows {
        for r in mapped {
            if let Ok(item) = r {
                history.push(item);
            }
        }
    }
    history.reverse();
    history
}

pub fn search_relevant_history(query: &str, limit: usize) -> Vec<String> {
    let conn = match init_db() {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let sanitized_query: String = query.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    
    if sanitized_query.trim().is_empty() {
        return get_recent_history(limit);
    }

    let fts_query = format!("{}*", sanitized_query.split_whitespace().collect::<Vec<&str>>().join(" "));

    let mut stmt = match conn.prepare(
        "SELECT intent, output FROM logs_fts WHERE logs_fts MATCH ?1 ORDER BY rank LIMIT ?2"
    ) {
        Ok(s) => s,
        Err(_) => return get_recent_history(limit),
    };

    let rows = stmt.query_map(rusqlite::params![fts_query, limit], |row| {
        let intent: String = row.get(0)?;
        let output: String = row.get(1)?;
        Ok(format!("[Memory RAG] User: {}\nAI: {}", intent, output))
    });

    let mut results = Vec::new();
    if let Ok(mapped) = rows {
        for r in mapped {
            if let Ok(item) = r {
                results.push(item);
            }
        }
    }

    if results.is_empty() {
        get_recent_history(limit)
    } else {
        results
    }
}

pub fn create_task(username: &str, intent: &str) -> String {
    let task_id = Uuid::new_v4().to_string();
    if let Ok(conn) = init_db() {
        let _ = conn.execute(
            "INSERT INTO tasks (id, username, intent, status, output) VALUES (?1, ?2, ?3, 'Queued', '')",
            rusqlite::params![task_id, username, intent],
        );
    }
    task_id
}

pub fn get_next_queued_task() -> Option<(String, String, String)> {
    let conn = init_db().ok()?;
    let tx = conn.unchecked_transaction().ok()?;
    
    let task = {
        let mut stmt = tx.prepare("SELECT id, username, intent FROM tasks WHERE status = 'Queued' LIMIT 1").ok()?;
        stmt.query_row([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        }).ok()?
    }; // Стейтмент stmt здесь падает из области видимости и освобождает заем tx

    let _ = tx.execute("UPDATE tasks SET status = 'Running' WHERE id = ?1", rusqlite::params![task.0]);
    let _ = tx.commit();
    Some(task)
}

pub fn update_task_status(id: &str, status: &str, output: &str) {
    if let Ok(conn) = init_db() {
        let _ = conn.execute(
            "UPDATE tasks SET status = ?1, output = ?2 WHERE id = ?3",
            rusqlite::params![status, output, id],
        );
    }
}

pub fn get_task(id: &str) -> Option<(String, String, String)> {
    let conn = init_db().ok()?;
    let mut stmt = conn.prepare("SELECT status, output, intent FROM tasks WHERE id = ?1").ok()?;
    stmt.query_row(rusqlite::params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    }).ok()
}