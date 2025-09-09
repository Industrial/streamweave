use http::StatusCode;

/// HTTP error types for better error handling
#[derive(Debug, Clone, PartialEq)]
pub enum HttpErrorType {
  NotFound,
  Unauthorized,
  Forbidden,
  InternalServerError,
  BadRequest,
  Timeout,
  RateLimited,
  MethodNotAllowed,
  Conflict,
  UnprocessableEntity,
  TooManyRequests,
  ServiceUnavailable,
}

impl HttpErrorType {
  pub fn status_code(&self) -> StatusCode {
    match self {
      HttpErrorType::NotFound => StatusCode::NOT_FOUND,
      HttpErrorType::Unauthorized => StatusCode::UNAUTHORIZED,
      HttpErrorType::Forbidden => StatusCode::FORBIDDEN,
      HttpErrorType::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
      HttpErrorType::BadRequest => StatusCode::BAD_REQUEST,
      HttpErrorType::Timeout => StatusCode::REQUEST_TIMEOUT,
      HttpErrorType::RateLimited => StatusCode::TOO_MANY_REQUESTS,
      HttpErrorType::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
      HttpErrorType::Conflict => StatusCode::CONFLICT,
      HttpErrorType::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
      HttpErrorType::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
      HttpErrorType::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
  }

  pub fn default_message(&self) -> &'static str {
    match self {
      HttpErrorType::NotFound => "Not Found",
      HttpErrorType::Unauthorized => "Unauthorized",
      HttpErrorType::Forbidden => "Forbidden",
      HttpErrorType::InternalServerError => "Internal Server Error",
      HttpErrorType::BadRequest => "Bad Request",
      HttpErrorType::Timeout => "Request Timeout",
      HttpErrorType::RateLimited => "Too Many Requests",
      HttpErrorType::MethodNotAllowed => "Method Not Allowed",
      HttpErrorType::Conflict => "Conflict",
      HttpErrorType::UnprocessableEntity => "Unprocessable Entity",
      HttpErrorType::TooManyRequests => "Too Many Requests",
      HttpErrorType::ServiceUnavailable => "Service Unavailable",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_http_error_types() {
    assert_eq!(HttpErrorType::NotFound.status_code(), StatusCode::NOT_FOUND);
    assert_eq!(
      HttpErrorType::Unauthorized.status_code(),
      StatusCode::UNAUTHORIZED
    );
    assert_eq!(
      HttpErrorType::BadRequest.status_code(),
      StatusCode::BAD_REQUEST
    );
  }

  #[test]
  fn test_http_error_messages() {
    assert_eq!(HttpErrorType::NotFound.default_message(), "Not Found");
    assert_eq!(
      HttpErrorType::Unauthorized.default_message(),
      "Unauthorized"
    );
    assert_eq!(HttpErrorType::BadRequest.default_message(), "Bad Request");
  }
}
