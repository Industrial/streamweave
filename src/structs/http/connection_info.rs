use http::Version;
use std::net::SocketAddr;

use crate::structs::http::tls_info::TlsInfo;

/// Connection information for HTTP requests
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
  pub remote_addr: SocketAddr,
  pub local_addr: SocketAddr,
  pub protocol: Version,
  pub tls_info: Option<TlsInfo>,
}

impl ConnectionInfo {
  pub fn new(remote_addr: SocketAddr, local_addr: SocketAddr, protocol: Version) -> Self {
    Self {
      remote_addr,
      local_addr,
      protocol,
      tls_info: None,
    }
  }

  pub fn with_tls(mut self, tls_info: TlsInfo) -> Self {
    self.tls_info = Some(tls_info);
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_connection_info() {
    let conn_info = ConnectionInfo::new(
      "127.0.0.1:8080".parse().unwrap(),
      "0.0.0.0:3000".parse().unwrap(),
      Version::HTTP_11,
    );

    assert_eq!(conn_info.remote_addr, "127.0.0.1:8080".parse().unwrap());
    assert_eq!(conn_info.local_addr, "0.0.0.0:3000".parse().unwrap());
    assert_eq!(conn_info.protocol, Version::HTTP_11);
    assert!(conn_info.tls_info.is_none());
  }
}
