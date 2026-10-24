use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type Slot<V> = Arc<tokio::sync::Mutex<Option<(Instant, V)>>>;

/// Answers worked out at most once per `ttl` for each key, shared by every
/// request: while one caller works out a stale answer, the others asking
/// for it wait for that one rather than asking too. Failures aren't kept.
pub struct Cache<K, V> {
    ttl: Duration,
    slots: Mutex<HashMap<K, Slot<V>>>,
}

impl<K: Eq + Hash, V: Clone> Cache<K, V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            slots: Mutex::default(),
        }
    }

    pub async fn get<E, F>(&self, key: K, work: impl FnOnce() -> F) -> Result<V, E>
    where
        F: Future<Output = Result<V, E>>,
    {
        let slot = {
            let mut slots = self.slots.lock().unwrap_or_else(|e| e.into_inner());
            // Forget stale answers nobody is waiting on.
            slots.retain(|_, slot| {
                Arc::strong_count(slot) > 1
                    || slot.try_lock().is_ok_and(
                        |held| matches!(&*held, Some((at, _)) if at.elapsed() < self.ttl),
                    )
            });
            slots.entry(key).or_default().clone()
        };
        let mut held = slot.lock().await;
        if let Some((at, value)) = &*held
            && at.elapsed() < self.ttl
        {
            return Ok(value.clone());
        }
        let value = work().await?;
        *held = Some((Instant::now(), value.clone()));
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn answers_are_worked_out_once_until_stale() {
        let cache = Cache::new(Duration::from_millis(50));
        let calls = AtomicU32::new(0);
        let ask = |key: u8| {
            cache.get(key, || async {
                Ok::<_, ()>(calls.fetch_add(1, Ordering::Relaxed))
            })
        };
        assert_eq!(ask(1).await, Ok(0));
        assert_eq!(ask(1).await, Ok(0));
        assert_eq!(ask(2).await, Ok(1));
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(ask(1).await, Ok(2));
        // Failures aren't kept.
        let failed = cache.get(3, || async { Err::<u32, _>("down") }).await;
        assert_eq!(failed, Err("down"));
        assert_eq!(ask(3).await, Ok(3));
    }
}
