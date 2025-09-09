use crate::http::http_request_chunk::StreamWeaveHttpRequestChunk;
use crate::input::Input;
use crate::output::Output;
use crate::transformer::Transformer;
use crate::transformer::TransformerConfig;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

/// Authentication strategy trait for different auth methods
#[async_trait]
pub trait AuthStrategy: Send + Sync {
  async fn authenticate(&self, request: &StreamWeaveHttpRequestChunk) -> AuthResult;
}

/// Authentication result
#[derive(Debug, Clone)]
pub enum AuthResult {
  Authenticated { user_id: String, roles: Vec<String> },
  Unauthenticated { reason: String },
  Error { message: String },
}

/// Bearer token authentication strategy
pub struct BearerTokenAuth {
  token_validator: Arc<dyn TokenValidator + Send + Sync>,
}

#[async_trait]
impl AuthStrategy for BearerTokenAuth {
  async fn authenticate(&self, request: &StreamWeaveHttpRequestChunk) -> AuthResult {
    let auth_header = request
      .headers
      .get("authorization")
      .and_then(|h| h.to_str().ok())
      .unwrap_or("");

    if !auth_header.starts_with("Bearer ") {
      return AuthResult::Unauthenticated {
        reason: "Missing or invalid authorization header".to_string(),
      };
    }

    let token = &auth_header[7..]; // Remove "Bearer " prefix
    match self.token_validator.validate(token).await {
      Ok(user_info) => AuthResult::Authenticated {
        user_id: user_info.user_id,
        roles: user_info.roles,
      },
      Err(e) => AuthResult::Error {
        message: e.to_string(),
      },
    }
  }
}

/// API key authentication strategy
pub struct ApiKeyAuth {
  valid_keys: HashMap<String, UserInfo>,
}

#[async_trait]
impl AuthStrategy for ApiKeyAuth {
  async fn authenticate(&self, request: &StreamWeaveHttpRequestChunk) -> AuthResult {
    let api_key = request
      .headers
      .get("x-api-key")
      .and_then(|h| h.to_str().ok())
      .unwrap_or("");

    if api_key.is_empty() {
      return AuthResult::Unauthenticated {
        reason: "Missing API key".to_string(),
      };
    }

    match self.valid_keys.get(api_key) {
      Some(user_info) => AuthResult::Authenticated {
        user_id: user_info.user_id.clone(),
        roles: user_info.roles.clone(),
      },
      None => AuthResult::Unauthenticated {
        reason: "Invalid API key".to_string(),
      },
    }
  }
}

/// User information extracted from authentication
#[derive(Debug, Clone)]
pub struct UserInfo {
  pub user_id: String,
  pub roles: Vec<String>,
}

/// Token validator trait
#[async_trait]
pub trait TokenValidator: Send + Sync {
  async fn validate(
    &self,
    token: &str,
  ) -> Result<UserInfo, Box<dyn std::error::Error + Send + Sync>>;
}

/// Simple JWT token validator (placeholder implementation)
#[allow(dead_code)]
pub struct JwtTokenValidator {
  secret: String,
}

#[async_trait]
impl TokenValidator for JwtTokenValidator {
  async fn validate(
    &self,
    _token: &str,
  ) -> Result<UserInfo, Box<dyn std::error::Error + Send + Sync>> {
    // In a real implementation, this would validate JWT tokens
    // For now, we'll just return a mock user
    Ok(UserInfo {
      user_id: "user123".to_string(),
      roles: vec!["user".to_string()],
    })
  }
}

/// Authentication transformer
pub struct AuthTransformer {
  auth_strategies: Arc<Vec<Box<dyn AuthStrategy>>>,
  config: TransformerConfig<StreamWeaveHttpRequestChunk>,
  required_roles: Vec<String>,
}

impl AuthTransformer {
  pub fn new() -> Self {
    Self {
      auth_strategies: Arc::new(Vec::new()),
      config: TransformerConfig::default(),
      required_roles: Vec::new(),
    }
  }

  pub fn with_strategy(mut self, strategy: Box<dyn AuthStrategy>) -> Self {
    Arc::get_mut(&mut self.auth_strategies)
      .unwrap()
      .push(strategy);
    self
  }

  pub fn with_required_roles(mut self, roles: Vec<String>) -> Self {
    self.required_roles = roles;
    self
  }

  pub fn with_config(mut self, config: TransformerConfig<StreamWeaveHttpRequestChunk>) -> Self {
    self.config = config;
    self
  }

  #[allow(dead_code)]
  async fn authenticate_request(
    &self,
    request: &StreamWeaveHttpRequestChunk,
  ) -> Result<UserInfo, AuthError> {
    // Try each strategy until one succeeds or we run out of strategies
    for strategy in self.auth_strategies.iter() {
      match strategy.authenticate(request).await {
        AuthResult::Authenticated { user_id, roles } => {
          // Check if user has required roles
          if !self.required_roles.is_empty() {
            let has_required_role = self
              .required_roles
              .iter()
              .any(|required_role| roles.contains(required_role));

            if !has_required_role {
              return Err(AuthError::InsufficientPermissions {
                required: self.required_roles.clone(),
                actual: roles,
              });
            }
          }

          return Ok(UserInfo { user_id, roles });
        }
        AuthResult::Unauthenticated { reason: _ } => {
          // Continue to next strategy
          continue;
        }
        AuthResult::Error { message: _ } => {
          // Continue to next strategy
          continue;
        }
      }
    }

    Err(AuthError::NoStrategy)
  }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
  #[error("Unauthenticated: {reason}")]
  Unauthenticated { reason: String },

  #[error("Insufficient permissions. Required: {required:?}, Actual: {actual:?}")]
  InsufficientPermissions {
    required: Vec<String>,
    actual: Vec<String>,
  },

  #[error("Validation error: {message}")]
  ValidationError { message: String },

  #[error("No authentication strategy available")]
  NoStrategy,
}

impl Default for AuthTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for AuthTransformer {
  type Input = StreamWeaveHttpRequestChunk;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for AuthTransformer {
  type Output = StreamWeaveHttpRequestChunk;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for AuthTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let strategies = Arc::clone(&self.auth_strategies);
    let required_roles = self.required_roles.clone();

    Box::pin(async_stream::stream! {
        let mut input_stream = input;

        while let Some(request) = input_stream.next().await {
            // Try to authenticate the request
            let mut authenticated = false;

            for strategy in strategies.iter() {
                match strategy.authenticate(&request).await {
                    AuthResult::Authenticated { user_id: _, roles } => {
                        // Check required roles
                        if !required_roles.is_empty() {
                            let has_required_role = required_roles.iter()
                                .any(|required_role| roles.contains(required_role));

                            if !has_required_role {
                                // Role check failed, continue to next strategy
                                continue;
                            }
                        }

                        // Authentication successful
                        authenticated = true;
                        break;
                    }
                    AuthResult::Unauthenticated { reason: _ } => {
                        // Try next strategy
                        continue;
                    }
                    AuthResult::Error { message: _ } => {
                        // Try next strategy
                        continue;
                    }
                }
            }

            // Only yield the request if authentication was successful
            if authenticated {
                yield request;
            }
            // Otherwise, skip this request (authentication failed)
        }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::http::{
    connection_info::ConnectionInfo, http_request_chunk::StreamWeaveHttpRequestChunk,
  };
  use http::{HeaderMap, Method, Uri, Version};
  use std::collections::HashMap;

  fn create_test_request(headers: HeaderMap) -> StreamWeaveHttpRequestChunk {
    let connection_info = ConnectionInfo::new(
      "127.0.0.1:8080".parse().unwrap(),
      "0.0.0.0:3000".parse().unwrap(),
      Version::HTTP_11,
    );

    StreamWeaveHttpRequestChunk::new(
      Method::GET,
      Uri::from_static("/test"),
      headers,
      bytes::Bytes::new(),
      connection_info,
      true,
    )
  }

  #[tokio::test]
  async fn test_bearer_token_auth_success() {
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer valid_token".parse().unwrap());

    let request = create_test_request(headers);
    let validator = Arc::new(JwtTokenValidator {
      secret: "test_secret".to_string(),
    });
    let auth = BearerTokenAuth {
      token_validator: validator,
    };

    let result = auth.authenticate(&request).await;
    match result {
      AuthResult::Authenticated { user_id, roles } => {
        assert_eq!(user_id, "user123");
        assert_eq!(roles, vec!["user"]);
      }
      _ => panic!("Expected authenticated result"),
    }
  }

  #[tokio::test]
  async fn test_bearer_token_auth_missing_header() {
    let headers = HeaderMap::new();
    let request = create_test_request(headers);
    let validator = Arc::new(JwtTokenValidator {
      secret: "test_secret".to_string(),
    });
    let auth = BearerTokenAuth {
      token_validator: validator,
    };

    let result = auth.authenticate(&request).await;
    match result {
      AuthResult::Unauthenticated { reason } => {
        assert!(reason.contains("Missing or invalid authorization header"));
      }
      _ => panic!("Expected unauthenticated result"),
    }
  }

  #[tokio::test]
  async fn test_api_key_auth_success() {
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", "valid_key".parse().unwrap());

    let request = create_test_request(headers);
    let mut valid_keys = HashMap::new();
    valid_keys.insert(
      "valid_key".to_string(),
      UserInfo {
        user_id: "user456".to_string(),
        roles: vec!["admin".to_string()],
      },
    );

    let auth = ApiKeyAuth { valid_keys };
    let result = auth.authenticate(&request).await;

    match result {
      AuthResult::Authenticated { user_id, roles } => {
        assert_eq!(user_id, "user456");
        assert_eq!(roles, vec!["admin"]);
      }
      _ => panic!("Expected authenticated result"),
    }
  }

  #[tokio::test]
  async fn test_api_key_auth_invalid_key() {
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", "invalid_key".parse().unwrap());

    let request = create_test_request(headers);
    let valid_keys = HashMap::new();
    let auth = ApiKeyAuth { valid_keys };
    let result = auth.authenticate(&request).await;

    match result {
      AuthResult::Unauthenticated { reason } => {
        assert_eq!(reason, "Invalid API key");
      }
      _ => panic!("Expected unauthenticated result"),
    }
  }

  #[tokio::test]
  async fn test_auth_transformer_with_roles() {
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", "admin_key".parse().unwrap());

    let request = create_test_request(headers);
    let mut valid_keys = HashMap::new();
    valid_keys.insert(
      "admin_key".to_string(),
      UserInfo {
        user_id: "admin".to_string(),
        roles: vec!["admin".to_string(), "user".to_string()],
      },
    );

    let auth_strategy = Box::new(ApiKeyAuth { valid_keys });
    let transformer = AuthTransformer::new()
      .with_strategy(auth_strategy)
      .with_required_roles(vec!["admin".to_string()]);

    // Test the authentication logic directly
    let result = transformer.authenticate_request(&request).await;
    assert!(result.is_ok());
    let user_info = result.unwrap();
    assert_eq!(user_info.user_id, "admin");
    assert!(user_info.roles.contains(&"admin".to_string()));
  }

  #[tokio::test]
  async fn test_auth_transformer_insufficient_permissions() {
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", "user_key".parse().unwrap());

    let request = create_test_request(headers);
    let mut valid_keys = HashMap::new();
    valid_keys.insert(
      "user_key".to_string(),
      UserInfo {
        user_id: "user".to_string(),
        roles: vec!["user".to_string()],
      },
    );

    let auth_strategy = Box::new(ApiKeyAuth { valid_keys });
    let transformer = AuthTransformer::new()
      .with_strategy(auth_strategy)
      .with_required_roles(vec!["admin".to_string()]);

    // Test the authentication logic directly
    let result = transformer.authenticate_request(&request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
      AuthError::InsufficientPermissions { required, actual } => {
        assert_eq!(required, vec!["admin"]);
        assert_eq!(actual, vec!["user"]);
      }
      _ => panic!("Expected insufficient permissions error"),
    }
  }
}
