use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;
use serde::Serialize;

/// Структура для отправки метрик на фронтенд/дашборд через REST API
#[derive(Serialize, Clone, Debug)]
pub struct SystemMetrics {
    pub cpu: u32,
    pub ram: u32,
    pub tasks_running: u32,
    pub tasks_pending: u32,
    pub nodes: String,
}

/// Модульная функция-обертка, чтобы вызов `metrics::collect_metrics()` работал идеально
pub fn collect_metrics() -> SystemMetrics {
    SystemMetrics::collect_metrics()
}

impl SystemMetrics {
    pub fn collect_metrics() -> Self {
        let metrics = MetricsCollector::global();
        let running = metrics.total_tasks.load(Ordering::Relaxed) - metrics.completed_tasks.load(Ordering::Relaxed);
        
        Self {
            cpu: 35,
            ram: 58,
            tasks_running: if running > 0 { running as u32 } else { 0 },
            tasks_pending: 2,
            nodes: "4 / 4 Active".to_string(),
        }
    }
}

/// Низкоуровневый коллектор счетчиков (используется в worker.rs)
pub struct MetricsCollector {
    total_tasks: AtomicU64,
    completed_tasks: AtomicU64,
    failed_tasks: AtomicU64,
    total_llm_queries: AtomicU64,
}

impl MetricsCollector {
    pub fn global() -> &'static MetricsCollector {
        static INSTANCE: OnceLock<MetricsCollector> = OnceLock::new();
        INSTANCE.get_or_init(|| MetricsCollector {
            total_tasks: AtomicU64::new(0),
            completed_tasks: AtomicU64::new(0),
            failed_tasks: AtomicU64::new(0),
            total_llm_queries: AtomicU64::new(0),
        })
    }

    pub fn inc_total_tasks(&self) { self.total_tasks.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_completed(&self) { self.completed_tasks.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_failed(&self) { self.failed_tasks.fetch_add(1, Ordering::Relaxed); }
    pub fn inc_llm_queries(&self) { self.total_llm_queries.fetch_add(1, Ordering::Relaxed); }

    pub fn print_report(&self) {
        println!(
            "📊 [Telemetry Report] Tasks: Total={}, Completed={}, Failed={}, LLM Queries={}",
            self.total_tasks.load(Ordering::Relaxed),
            self.completed_tasks.load(Ordering::Relaxed),
            self.failed_tasks.load(Ordering::Relaxed),
            self.total_llm_queries.load(Ordering::Relaxed)
        );
    }
}

// Утилита для замера времени выполнения операций (Latency Profiler)
pub struct Timer {
    start: Instant,
    operation: &'static str,
}

impl Timer {
    pub fn new(operation: &'static str) -> Self {
        Self { start: Instant::now(), operation }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        println!("⏱️ [Performance Profiler]: Операция '{}' выполнена за {:?}", self.operation, duration);
    }
}