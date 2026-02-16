use std::net::IpAddr;

use mdns_sd::{ServiceDaemon, ServiceInfo as MdnsServiceInfo};
use tokio_util::sync::CancellationToken;

use crate::DiscoveryError;
use crate::types::{SERVICE_NAME, ServiceInfo};

/// Advertises an agent on the local network via mDNS/DNS-SD.
pub struct Server {
    info: ServiceInfo,
    daemon: Option<ServiceDaemon>,
}

impl Server {
    /// Creates a new mDNS server for advertising an agent.
    pub fn new(info: ServiceInfo) -> Self {
        Self { info, daemon: None }
    }

    /// Begins advertising the agent on the network.
    ///
    /// The port must be set before calling start (no default).
    /// Safe to call multiple times: stops the existing daemon before re-registering.
    pub fn start(&mut self) -> Result<(), DiscoveryError> {
        if self.info.port == 0 {
            return Err(DiscoveryError::Config(
                "port must be set before starting mDNS server".into(),
            ));
        }

        // Stop existing daemon if running
        self.stop()?;

        let daemon = ServiceDaemon::new()
            .map_err(|e| DiscoveryError::Mdns(format!("failed to create mDNS daemon: {e}")))?;

        let ips = if self.info.ips.is_empty() {
            get_local_ips()
        } else {
            self.info.ips.clone()
        };

        if ips.is_empty() {
            return Err(DiscoveryError::Network("no valid network IPs found".into()));
        }

        let hostname = get_hostname();
        let service_type = format!("{SERVICE_NAME}.local.");
        let full_name = format!("{}.{service_type}", self.info.id);

        let properties = [
            ("id", self.info.id.as_str()),
            ("name", self.info.name.as_str()),
            ("platform", self.info.platform.as_str()),
            ("version", self.info.version.as_str()),
        ];

        let service = MdnsServiceInfo::new(
            &service_type,
            &self.info.id,
            &hostname,
            &ips[..],
            self.info.port,
            &properties[..],
        )
        .map_err(|e| DiscoveryError::Mdns(format!("failed to create service info: {e}")))?;

        daemon.register(service).map_err(|e| {
            DiscoveryError::Mdns(format!(
                "failed to register mDNS service '{full_name}': {e}"
            ))
        })?;

        self.daemon = Some(daemon);
        Ok(())
    }

    /// Stops advertising the agent.
    pub fn stop(&mut self) -> Result<(), DiscoveryError> {
        if let Some(daemon) = self.daemon.take() {
            let service_type = format!("{SERVICE_NAME}.local.");
            let full_name = format!("{}.{service_type}", self.info.id);
            let _ = daemon.unregister(&full_name);
            let _ = daemon.shutdown();
        }
        Ok(())
    }

    /// Returns the service info being advertised.
    pub fn info(&self) -> &ServiceInfo {
        &self.info
    }

    /// Starts the server and runs until the cancellation token is triggered.
    pub async fn run_context(&mut self, cancel: CancellationToken) -> Result<(), DiscoveryError> {
        self.start()?;
        cancel.cancelled().await;
        self.stop()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Returns local non-loopback IPv4 addresses, excluding link-local (169.254.x.x).
pub fn get_local_ips() -> Vec<IpAddr> {
    let mut ips = Vec::new();

    let Ok(interfaces) = if_addrs::get_if_addrs() else {
        return ips;
    };

    for iface in interfaces {
        if iface.is_loopback() {
            continue;
        }
        if let IpAddr::V4(ipv4) = iface.ip() {
            // Skip loopback
            if ipv4.octets()[0] == 127 {
                continue;
            }
            // Skip link-local (169.254.x.x / APIPA)
            if ipv4.octets()[0] == 169 && ipv4.octets()[1] == 254 {
                continue;
            }
            ips.push(IpAddr::V4(ipv4));
        }
    }

    ips
}

/// Returns the local hostname suffixed with `.local.` as required by mDNS.
pub fn get_hostname() -> String {
    let mut name = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".into());

    if !name.ends_with(".local.") {
        name = name.trim_end_matches('.').to_string();
        name.push_str(".local.");
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_requires_port() {
        let info = ServiceInfo {
            id: "test".into(),
            name: "Test".into(),
            platform: "linux".into(),
            version: "0.1.0".into(),
            port: 0,
            ips: vec![],
        };
        let mut server = Server::new(info);
        assert!(server.start().is_err());
    }

    #[test]
    fn get_hostname_returns_something() {
        let h = get_hostname();
        assert!(!h.is_empty());
    }

    #[test]
    fn server_info_access() {
        let info = ServiceInfo {
            id: "test".into(),
            name: "Test Agent".into(),
            platform: "linux".into(),
            version: "0.1.0".into(),
            port: 8765,
            ips: vec![],
        };
        let server = Server::new(info.clone());
        assert_eq!(server.info().id, "test");
        assert_eq!(server.info().name, "Test Agent");
    }
}
