use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// A value fetched from the database and kept for a while, so a busy page
/// doesn't ask for it on every visit. Whatever changes it calls
/// [`Cached::clear`]; a change made elsewhere (by hand in SQL) shows once
/// it's been kept for `keep_for`.
pub struct Cached<T> {
    keep_for: Duration,
    state: Mutex<State<T>>,
}

struct State<T> {
    value: Option<(Instant, Arc<T>)>,
    cleared: u64,
}

impl<T> Cached<T> {
    pub fn new(keep_for: Duration) -> Self {
        Self {
            keep_for,
            state: Mutex::new(State {
                value: None,
                cleared: 0,
            }),
        }
    }

    /// The kept value, or `fetch`'s if there's none or it's too old. A
    /// failed fetch isn't kept, so the next visit tries again.
    pub async fn get<E>(&self, fetch: impl AsyncFnOnce() -> Result<T, E>) -> Result<Arc<T>, E> {
        let cleared = {
            let state = self.lock();
            if let Some((at, value)) = &state.value
                && at.elapsed() < self.keep_for
            {
                return Ok(value.clone());
            }
            state.cleared
        };
        let value = Arc::new(fetch().await?);
        let mut state = self.lock();
        if state.cleared == cleared {
            state.value = Some((Instant::now(), value.clone()));
        }
        Ok(value)
    }

    /// Drops the kept value: the next [`Cached::get`] fetches it afresh.
    pub fn clear(&self) {
        let mut state = self.lock();
        state.value = None;
        state.cleared += 1;
    }

    fn lock(&self) -> MutexGuard<'_, State<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    #[tokio::test]
    async fn keeps_a_value_until_cleared() {
        let cached = Cached::new(Duration::from_secs(60));
        let got = cached.get(async || Ok::<_, Infallible>(1)).await.unwrap();
        assert_eq!(*got, 1);
        let got = cached.get(async || Ok::<_, Infallible>(2)).await.unwrap();
        assert_eq!(*got, 1);
        cached.clear();
        let got = cached.get(async || Ok::<_, Infallible>(3)).await.unwrap();
        assert_eq!(*got, 3);
    }

    #[tokio::test]
    async fn refetches_when_too_old_or_failed() {
        let cached = Cached::new(Duration::ZERO);
        cached.get(async || Ok::<_, Infallible>(1)).await.unwrap();
        let got = cached.get(async || Ok::<_, Infallible>(2)).await.unwrap();
        assert_eq!(*got, 2);

        let cached = Cached::new(Duration::from_secs(60));
        assert!(cached.get(async || Err(())).await.is_err());
        let got = cached.get(async || Ok::<_, ()>(4)).await.unwrap();
        assert_eq!(*got, 4);
    }

    #[tokio::test]
    async fn a_fetch_from_before_a_clear_isnt_kept() {
        let cached = Cached::new(Duration::from_secs(60));
        let got = cached
            .get(async || {
                // News changes while the old posts are on their way.
                cached.clear();
                Ok::<_, Infallible>(1)
            })
            .await
            .unwrap();
        assert_eq!(*got, 1);
        let got = cached.get(async || Ok::<_, Infallible>(2)).await.unwrap();
        assert_eq!(*got, 2);
    }
}
