use crate::http::route_pattern::RoutePattern;

/// Route definition for the router
#[derive(Debug, Clone)]
pub struct RouteDefinition {
  pub pattern: RoutePattern,
  pub handler: String,         // Handler identifier
  pub middleware: Vec<String>, // Middleware identifiers
}

impl RouteDefinition {
  pub fn new(pattern: RoutePattern, handler: String) -> Self {
    Self {
      pattern,
      handler,
      middleware: Vec::new(),
    }
  }

  pub fn with_middleware(mut self, middleware: Vec<String>) -> Self {
    self.middleware = middleware;
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::http::route_pattern::RoutePattern;
  use http::Method;

  #[test]
  fn test_route_definition() {
    let pattern = RoutePattern::new(Method::GET, "/users/{id}").unwrap();
    let route = RouteDefinition::new(pattern, "user_handler".to_string())
      .with_middleware(vec!["auth".to_string(), "logging".to_string()]);

    assert_eq!(route.handler, "user_handler");
    assert_eq!(route.middleware, vec!["auth", "logging"]);
  }
}
