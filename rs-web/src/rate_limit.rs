use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Addresses tracked before expired ones are cleared out.
const PRUNE_AT: usize = 10_000;

pub struct RateLimit {
    max: u32,
    window: Duration,
    seen: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl RateLimit {
    /// `max` requests from one address per `window`.
    pub fn new(max: u32, window: Duration) -> Self {
        Self {
            max,
            window,
            seen: Mutex::default(),
        }
    }

    /// Counts a request from `ip`: whether it's within the limit.
    pub fn allow(&self, ip: IpAddr) -> bool {
        self.allow_at(ip, Instant::now())
    }

    fn allow_at(&self, ip: IpAddr, now: Instant) -> bool {
        let Ok(mut seen) = self.seen.lock() else {
            return true;
        };
        if seen.len() >= PRUNE_AT {
            seen.retain(|_, (start, _)| now.duration_since(*start) < self.window);
        }
        let (start, count) = seen.entry(address(ip)).or_insert((now, 0));
        if now.duration_since(*start) >= self.window {
            *start = now;
            *count = 0;
        }
        *count += 1;
        *count <= self.max
    }
}

/// The address a limit counts `ip` as: an IPv6 address with the rest of its
/// /64, which one household or phone is handed whole.
fn address(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => IpAddr::V6((v6.to_bits() & u128::MAX << 64).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_each_address_per_window() {
        let limit = RateLimit::new(2, Duration::from_secs(60));
        let ip: IpAddr = "203.0.113.7".parse().unwrap();
        let other: IpAddr = "203.0.113.8".parse().unwrap();
        let t = Instant::now();
        assert!(limit.allow_at(ip, t));
        assert!(limit.allow_at(ip, t));
        assert!(!limit.allow_at(ip, t + Duration::from_secs(59)));
        assert!(limit.allow_at(other, t));
        // A new window starts afresh.
        assert!(limit.allow_at(ip, t + Duration::from_secs(60)));
    }

    #[test]
    fn ipv6_counts_by_64() {
        let limit = RateLimit::new(1, Duration::from_secs(60));
        let t = Instant::now();
        assert!(limit.allow_at("2001:db8::1".parse().unwrap(), t));
        assert!(!limit.allow_at("2001:db8::ffff:2".parse().unwrap(), t));
        assert!(limit.allow_at("2001:db8:0:1::1".parse().unwrap(), t));
    }
}
