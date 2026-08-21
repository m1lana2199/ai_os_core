use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::time::{Instant, Duration};

pub struct CircuitBreaker {
    failure_threshold: u32,
    cooldown_duration: Duration,
    failures: AtomicU32,
    is_open: AtomicBool,
    last_failure_time: std::sync::Mutex<Option<Instant>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown_secs: u64) -> Self {
        Self {
            failure_threshold,
            cooldown_duration: Duration::from_secs(cooldown_secs),
            failures: AtomicU32::new(0),
            is_open: AtomicBool::new(false),
            last_failure_time: std::sync::Mutex::new(None),
        }
    }

    pub fn can_execute(&self) -> bool {
        if self.is_open.load(Ordering::Relaxed) {
            let mut lock = self.last_failure_time.lock().unwrap();
            if let Some(time) = *lock {
                if time.elapsed() > self.cooldown_duration {
                    // Самовосстановление (Half-Open -> Closed)
                    self.is_open.store(false, Ordering::Relaxed);
                    self.failures.store(0, Ordering::Relaxed);
                    *lock = None;
                    println!("🔄 [Circuit Breaker]: Узел самовосстановился. Предохранитель закрыт.");
                    return true;
                }
            }
            return false;
        }
        true
    }

    pub fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        self.is_open.store(false, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        let current = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        if current >= self.failure_threshold {
            self.is_open.store(true, Ordering::Relaxed);
            let mut lock = self.last_failure_time.lock().unwrap();
            *lock = Some(Instant::now());
            println!("🚨 [Circuit Breaker]: Предохранитель РАЗОМКНУТ! LLM-контур изорован на {} сек.", self.cooldown_duration.as_secs());
        }
    }
}