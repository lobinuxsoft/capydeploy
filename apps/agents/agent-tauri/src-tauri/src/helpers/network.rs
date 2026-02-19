/// Returns local non-loopback IPv4 addresses, excluding link-local (169.254.x.x).
pub(crate) fn local_ips() -> Vec<String> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| {
            if iface.is_loopback() {
                return None;
            }
            match iface.addr.ip() {
                std::net::IpAddr::V4(ip) => {
                    // Skip link-local (169.254.x.x)
                    if ip.octets()[0] == 169 && ip.octets()[1] == 254 {
                        return None;
                    }
                    Some(ip.to_string())
                }
                _ => None,
            }
        })
        .collect()
}
