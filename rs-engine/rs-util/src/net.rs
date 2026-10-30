use std::net::IpAddr;

/// The address a per-address limit counts `ip` as: an IPv6 address with the
/// rest of its /64, which one household or phone is handed whole. An
/// IPv4-mapped address counts as its IPv4 address, or every IPv4 client of a
/// dual-stack socket would share one /64.
pub fn limit_address(ip: IpAddr) -> IpAddr {
    match ip.to_canonical() {
        IpAddr::V6(v6) => IpAddr::V6((v6.to_bits() & u128::MAX << 64).into()),
        v4 => v4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(ip: &str) -> String {
        limit_address(ip.parse().unwrap()).to_string()
    }

    #[test]
    fn ipv4_is_its_own_address() {
        assert_eq!(key("203.0.113.7"), "203.0.113.7");
        assert_ne!(key("203.0.113.7"), key("203.0.113.8"));
    }

    #[test]
    fn ipv6_counts_by_64() {
        assert_eq!(key("2001:db8::1"), "2001:db8::");
        assert_eq!(key("2001:db8::1"), key("2001:db8::ffff:2"));
        assert_ne!(key("2001:db8::1"), key("2001:db8:0:1::1"));
    }

    #[test]
    fn ipv4_mapped_counts_as_ipv4() {
        assert_eq!(key("::ffff:203.0.113.7"), "203.0.113.7");
        assert_ne!(key("::ffff:203.0.113.7"), key("::ffff:203.0.113.8"));
    }
}
