use http::Method;
use regex::Regex;
use std::collections::HashMap;

/// Route pattern for matching HTTP requests
#[derive(Debug, Clone)]
pub struct RoutePattern {
  pub method: Method,
  pub path: String,
  pub path_params: Vec<String>, // e.g., ["id", "name"]
  pub regex: Option<Regex>,
  pub priority: u32, // Higher number = higher priority
}

impl PartialEq for RoutePattern {
  fn eq(&self, other: &Self) -> bool {
    self.method == other.method
      && self.path == other.path
      && self.path_params == other.path_params
      && self.priority == other.priority
      && match (&self.regex, &other.regex) {
        (Some(r1), Some(r2)) => r1.as_str() == r2.as_str(),
        (None, None) => true,
        _ => false,
      }
  }
}

impl Eq for RoutePattern {}

impl std::hash::Hash for RoutePattern {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.method.hash(state);
    self.path.hash(state);
    self.path_params.hash(state);
    self.priority.hash(state);
    match &self.regex {
      Some(regex) => regex.as_str().hash(state),
      None => 0_u8.hash(state),
    }
  }
}

impl RoutePattern {
  /// Create a new route pattern from a path string
  /// Supports path parameters in the format `{param_name}`
  /// Example: "/users/{id}/posts/{post_id}" -> ["id", "post_id"]
  pub fn new(method: Method, path: &str) -> Result<Self, RoutePatternError> {
    let (path_params, regex_pattern) = Self::parse_path_params(path)?;
    let regex = if path_params.is_empty() {
      None
    } else {
      Some(Regex::new(&regex_pattern)?)
    };

    Ok(Self {
      method,
      path: path.to_string(),
      path_params,
      regex,
      priority: Self::calculate_priority(path),
    })
  }

  /// Parse path parameters from a path string
  fn parse_path_params(path: &str) -> Result<(Vec<String>, String), RoutePatternError> {
    let mut params = Vec::new();
    let mut regex_pattern = String::new();
    let chars = path.chars().peekable();
    let mut in_param = false;
    let mut param_name = String::new();

    for ch in chars {
      match ch {
        '{' if !in_param => {
          in_param = true;
          param_name.clear();
        }
        '}' if in_param => {
          if param_name.is_empty() {
            return Err(RoutePatternError::EmptyParameter);
          }
          params.push(param_name.clone());
          regex_pattern.push_str("([^/]+)");
          in_param = false;
        }
        _ if in_param => {
          if !ch.is_alphanumeric() && ch != '_' {
            return Err(RoutePatternError::InvalidParameterName(ch));
          }
          param_name.push(ch);
        }
        _ => {
          regex_pattern.push(ch);
        }
      }
    }

    if in_param {
      return Err(RoutePatternError::UnclosedParameter);
    }

    Ok((params, format!("^{}$", regex_pattern)))
  }

  /// Calculate priority based on path specificity
  fn calculate_priority(path: &str) -> u32 {
    let param_count = path.matches('{').count();
    let static_segments = path
      .split('/')
      .filter(|s| !s.is_empty() && !s.starts_with('{'))
      .count();

    // Higher priority for more static segments, lower for more parameters
    (static_segments as u32) * 1000 - (param_count as u32)
  }

  /// Match a request against this route pattern
  pub fn matches(&self, method: &Method, path: &str) -> Option<HashMap<String, String>> {
    if self.method != method {
      return None;
    }

    if self.path_params.is_empty() {
      return if self.path == path {
        Some(HashMap::new())
      } else {
        None
      };
    }

    if let Some(ref regex) = self.regex
      && let Some(captures) = regex.captures(path)
    {
      let mut params = HashMap::new();
      for (i, param_name) in self.path_params.iter().enumerate() {
        if let Some(value) = captures.get(i + 1) {
          params.insert(param_name.clone(), value.as_str().to_string());
        }
      }
      return Some(params);
    }

    None
  }

  /// Check if this pattern conflicts with another
  pub fn conflicts_with(&self, other: &RoutePattern) -> bool {
    if self.method != other.method {
      return false;
    }

    // If both have no parameters, they conflict if paths are equal
    if self.path_params.is_empty() && other.path_params.is_empty() {
      return self.path == other.path;
    }

    // If one has parameters and the other doesn't, they might conflict
    if self.path_params.is_empty() != other.path_params.is_empty() {
      return self.path == other.path;
    }

    // Both have parameters - check if they could match the same path
    if let (Some(regex1), Some(regex2)) = (&self.regex, &other.regex) {
      // This is a simplified check - in practice, you'd need more sophisticated logic
      return regex1.as_str() == regex2.as_str();
    }

    false
  }
}

/// Error types for route pattern parsing
#[derive(Debug, Clone, PartialEq)]
pub enum RoutePatternError {
  EmptyParameter,
  InvalidParameterName(char),
  UnclosedParameter,
  RegexError(String),
}

impl std::fmt::Display for RoutePatternError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RoutePatternError::EmptyParameter => write!(f, "Empty parameter in route pattern"),
      RoutePatternError::InvalidParameterName(ch) => {
        write!(f, "Invalid character '{}' in parameter name", ch)
      }
      RoutePatternError::UnclosedParameter => write!(f, "Unclosed parameter in route pattern"),
      RoutePatternError::RegexError(msg) => write!(f, "Regex error: {}", msg),
    }
  }
}

impl std::error::Error for RoutePatternError {}

impl From<regex::Error> for RoutePatternError {
  fn from(err: regex::Error) -> Self {
    RoutePatternError::RegexError(err.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use http::Method;

  #[test]
  fn test_route_pattern_creation() {
    let pattern = RoutePattern::new(Method::GET, "/users/{id}").unwrap();
    assert_eq!(pattern.method, Method::GET);
    assert_eq!(pattern.path, "/users/{id}");
    assert_eq!(pattern.path_params, vec!["id"]);
    assert!(pattern.regex.is_some());
  }

  #[test]
  fn test_route_pattern_matching() {
    let pattern = RoutePattern::new(Method::GET, "/users/{id}").unwrap();

    // Should match
    let params = pattern.matches(&Method::GET, "/users/123").unwrap();
    assert_eq!(params.get("id"), Some(&"123".to_string()));

    // Should not match different method
    assert!(pattern.matches(&Method::POST, "/users/123").is_none());

    // Should not match different path
    assert!(pattern.matches(&Method::GET, "/posts/123").is_none());
  }

  #[test]
  fn test_route_pattern_multiple_params() {
    let pattern = RoutePattern::new(Method::GET, "/users/{user_id}/posts/{post_id}").unwrap();
    assert_eq!(pattern.path_params, vec!["user_id", "post_id"]);

    let params = pattern
      .matches(&Method::GET, "/users/123/posts/456")
      .unwrap();
    assert_eq!(params.get("user_id"), Some(&"123".to_string()));
    assert_eq!(params.get("post_id"), Some(&"456".to_string()));
  }

  #[test]
  fn test_route_pattern_static_path() {
    let pattern = RoutePattern::new(Method::GET, "/api/health").unwrap();
    assert!(pattern.path_params.is_empty());
    assert!(pattern.regex.is_none());

    // Should match exact path
    assert!(pattern.matches(&Method::GET, "/api/health").is_some());

    // Should not match different path
    assert!(pattern.matches(&Method::GET, "/api/status").is_none());
  }

  #[test]
  fn test_route_pattern_errors() {
    // Empty parameter
    assert!(matches!(
      RoutePattern::new(Method::GET, "/users/{}"),
      Err(RoutePatternError::EmptyParameter)
    ));

    // Invalid parameter name
    assert!(matches!(
      RoutePattern::new(Method::GET, "/users/{id-param}"),
      Err(RoutePatternError::InvalidParameterName('-'))
    ));

    // Unclosed parameter
    assert!(matches!(
      RoutePattern::new(Method::GET, "/users/{id"),
      Err(RoutePatternError::UnclosedParameter)
    ));
  }

  #[test]
  fn test_route_pattern_priority() {
    let static_pattern = RoutePattern::new(Method::GET, "/api/users").unwrap();
    let param_pattern = RoutePattern::new(Method::GET, "/api/{resource}").unwrap();

    assert!(static_pattern.priority > param_pattern.priority);
  }

  #[test]
  fn test_route_pattern_conflicts() {
    let pattern1 = RoutePattern::new(Method::GET, "/users/{id}").unwrap();
    let pattern2 = RoutePattern::new(Method::GET, "/users/{id}").unwrap();
    let pattern3 = RoutePattern::new(Method::POST, "/users/{id}").unwrap();
    let pattern4 = RoutePattern::new(Method::GET, "/posts/{id}").unwrap();

    assert!(pattern1.conflicts_with(&pattern2));
    assert!(!pattern1.conflicts_with(&pattern3)); // Different method
    assert!(!pattern1.conflicts_with(&pattern4)); // Different path
  }
}
