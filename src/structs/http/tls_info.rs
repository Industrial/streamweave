/// TLS connection information
#[derive(Debug, Clone)]
pub struct TlsInfo {
  pub protocol: String,
  pub cipher_suite: Option<String>,
  pub peer_certificate: Option<Vec<u8>>,
}

impl TlsInfo {
  pub fn new(protocol: String) -> Self {
    Self {
      protocol,
      cipher_suite: None,
      peer_certificate: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_tls_info() {
    let tls_info = TlsInfo::new("TLSv1.3".to_string());
    assert_eq!(tls_info.protocol, "TLSv1.3");
    assert!(tls_info.cipher_suite.is_none());
    assert!(tls_info.peer_certificate.is_none());
  }
}
