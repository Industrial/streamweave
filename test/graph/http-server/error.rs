#[cfg(test)]
mod tests {
  use axum::http::StatusCode;
  use chrono::Utc;
  use std::io;
  use streamweave::error::{ComponentInfo, ErrorContext, StreamError};
  use streamweave_http_server::error::{
    ErrorDetails, ErrorResponse, create_custom_error, is_development_mode, map_generic_error,
    map_to_http_error,
  };

  #[test]
  fn test_error_response_serialization() {
    let error = ErrorResponse {
      status: 400,
      message: "Invalid input".to_string(),
      error: Some("ValidationError".to_string()),
      details: None,
    };

    let json = serde_json::to_string(&error).unwrap();
    assert!(json.contains("Invalid input"));
    assert!(json.contains("400"));
  }

  #[test]
  fn test_error_details_serialization() {
    let details = ErrorDetails {
      error: "Something went wrong".to_string(),
      component: Some("test_component".to_string()),
      timestamp: Some("2024-01-01T00:00:00Z".to_string()),
      context: Some("test context".to_string()),
    };

    let json = serde_json::to_string(&details).unwrap();
    assert!(json.contains("Something went wrong"));
  }

  #[test]
  fn test_map_to_http_error_not_found() {
    let error = StreamError::new(
      Box::new(io::Error::other("Resource not found")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert_eq!(http_error.status, StatusCode::NOT_FOUND.as_u16());
    assert_eq!(http_error.error, Some("NotFoundError".to_string()));
  }

  #[test]
  fn test_map_to_http_error_unauthorized() {
    let error = StreamError::new(
      Box::new(io::Error::other("Unauthorized access")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert_eq!(http_error.status, StatusCode::UNAUTHORIZED.as_u16());
    assert_eq!(http_error.error, Some("UnauthorizedError".to_string()));
  }

  #[test]
  fn test_map_to_http_error_forbidden() {
    let error = StreamError::new(
      Box::new(io::Error::other("Forbidden: permission denied")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert_eq!(http_error.status, StatusCode::FORBIDDEN.as_u16());
    assert_eq!(http_error.error, Some("ForbiddenError".to_string()));
  }

  #[test]
  fn test_map_to_http_error_rate_limit() {
    let error = StreamError::new(
      Box::new(io::Error::other("Rate limit exceeded")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert_eq!(http_error.status, StatusCode::TOO_MANY_REQUESTS.as_u16());
    assert_eq!(http_error.error, Some("RateLimitError".to_string()));
  }

  #[test]
  fn test_map_to_http_error_validation() {
    let error = StreamError::new(
      Box::new(io::Error::other("Invalid input validation failed")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert_eq!(http_error.status, StatusCode::BAD_REQUEST.as_u16());
    assert_eq!(http_error.error, Some("ValidationError".to_string()));
  }

  #[test]
  fn test_map_to_http_error_conflict() {
    let error = StreamError::new(
      Box::new(io::Error::other("Resource already exists conflict")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert_eq!(http_error.status, StatusCode::CONFLICT.as_u16());
    assert_eq!(http_error.error, Some("ConflictError".to_string()));
  }

  #[test]
  fn test_map_to_http_error_timeout() {
    let error = StreamError::new(
      Box::new(io::Error::other("Request timed out")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert_eq!(http_error.status, StatusCode::REQUEST_TIMEOUT.as_u16());
    assert_eq!(http_error.error, Some("TimeoutError".to_string()));
  }

  #[test]
  fn test_map_to_http_error_not_implemented() {
    let error = StreamError::new(
      Box::new(io::Error::other("Feature not implemented")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert_eq!(http_error.status, StatusCode::NOT_IMPLEMENTED.as_u16());
    assert_eq!(http_error.error, Some("NotImplementedError".to_string()));
  }

  #[test]
  fn test_map_to_http_error_service_unavailable() {
    let error = StreamError::new(
      Box::new(io::Error::other("Service unavailable")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert_eq!(http_error.status, StatusCode::SERVICE_UNAVAILABLE.as_u16());
    assert_eq!(
      http_error.error,
      Some("ServiceUnavailableError".to_string())
    );
  }

  #[test]
  fn test_map_to_http_error_default() {
    let error = StreamError::new(
      Box::new(io::Error::other("Generic error")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert_eq!(
      http_error.status,
      StatusCode::INTERNAL_SERVER_ERROR.as_u16()
    );
    assert_eq!(http_error.error, Some("InternalServerError".to_string()));
  }

  #[test]
  fn test_map_to_http_error_with_details() {
    let error = StreamError::new(
      Box::new(io::Error::other("Test error")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test_component".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, true);
    assert!(http_error.details.is_some());
    let details = http_error.details.unwrap();
    assert_eq!(details.component, Some("test_component".to_string()));
    assert!(details.timestamp.is_some());
  }

  #[test]
  fn test_map_to_http_error_without_details() {
    let error = StreamError::new(
      Box::new(io::Error::other("Test error")),
      ErrorContext {
        timestamp: Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestComponent".to_string(),
      },
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );

    let http_error = map_to_http_error(&error, false);
    assert!(http_error.details.is_none());
  }

  #[test]
  fn test_map_generic_error() {
    let error = io::Error::other("Generic error");
    let http_error = map_generic_error(&error, Some(StatusCode::BAD_REQUEST), false);

    assert_eq!(http_error.status, StatusCode::BAD_REQUEST.as_u16());
    assert_eq!(http_error.error, Some("Error".to_string()));
  }

  #[test]
  fn test_map_generic_error_default_status() {
    let error = io::Error::other("Generic error");
    let http_error = map_generic_error(&error, None, false);

    assert_eq!(
      http_error.status,
      StatusCode::INTERNAL_SERVER_ERROR.as_u16()
    );
  }

  #[test]
  fn test_map_generic_error_with_details() {
    let error = io::Error::other("Generic error");
    let http_error = map_generic_error(&error, None, true);

    assert!(http_error.details.is_some());
  }

  #[test]
  fn test_create_custom_error() {
    let error = create_custom_error(
      StatusCode::BAD_REQUEST,
      "Custom error message",
      Some("CustomError".to_string()),
      false,
    );

    assert_eq!(error.status, StatusCode::BAD_REQUEST.as_u16());
    assert_eq!(error.message, "Custom error message");
    assert_eq!(error.error, Some("CustomError".to_string()));
  }

  #[test]
  fn test_create_custom_error_with_details() {
    let error = create_custom_error(StatusCode::BAD_REQUEST, "Custom error", None, true);

    assert!(error.details.is_some());
  }

  #[test]
  fn test_is_development_mode() {
    // Clear environment variables to test default behavior
    std::env::remove_var("RUST_ENV");
    std::env::remove_var("ENVIRONMENT");

    // Should return false by default
    assert!(!is_development_mode());
  }

  #[test]
  fn test_is_development_mode_with_rust_env() {
    std::env::set_var("RUST_ENV", "development");
    assert!(is_development_mode());

    std::env::set_var("RUST_ENV", "dev");
    assert!(is_development_mode());

    std::env::set_var("RUST_ENV", "production");
    assert!(!is_development_mode());

    // Clean up
    std::env::remove_var("RUST_ENV");
  }

  #[test]
  fn test_is_development_mode_with_environment() {
    std::env::remove_var("RUST_ENV");
    std::env::set_var("ENVIRONMENT", "development");
    assert!(is_development_mode());

    std::env::set_var("ENVIRONMENT", "dev");
    assert!(is_development_mode());

    std::env::set_var("ENVIRONMENT", "production");
    assert!(!is_development_mode());

    // Clean up
    std::env::remove_var("ENVIRONMENT");
  }
}
