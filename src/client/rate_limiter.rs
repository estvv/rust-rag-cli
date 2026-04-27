// src/client/rate_limiter.rs

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32, _burst_size: u32) -> Self {
        let permits = requests_per_second.max(1) as usize;
        let semaphore = Arc::new(Semaphore::new(permits));

        let sem_clone = semaphore.clone();
        tokio::spawn(async move {
            let interval = Duration::from_millis(1000 / requests_per_second.max(1) as u64);
            loop {
                tokio::time::sleep(interval).await;
                sem_clone.add_permits(1);
            }
        });

        Self { semaphore }
    }

    pub fn disabled() -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(1000)),
        }
    }

    pub async fn acquire(&self) {
        let _ = self.semaphore.acquire().await;
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::disabled()
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            semaphore: self.semaphore.clone(),
        }
    }
}
