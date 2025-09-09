use crate::http::{
    http_request_chunk::StreamWeaveHttpRequestChunk,
    http_response::StreamWeaveHttpResponse,
};
use crate::transformer::Transformer;
use crate::input::Input;
use crate::output::Output;
use crate::transformer::TransformerConfig;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// Validation rule for HTTP requests
pub enum ValidationRule {
    /// Validate required headers
    RequiredHeaders(Vec<String>),
    /// Validate header values match pattern
    HeaderPattern(String, String), // header_name, regex_pattern
    /// Validate content type
    ContentType(String),
    /// Validate body size limits
    MaxBodySize(usize),
    /// Validate JSON schema
    JsonSchema(String), // JSON schema as string
    /// Custom validation function
    Custom(Box<dyn ValidationFunction + Send + Sync>),
}

/// Custom validation function trait
#[async_trait]
pub trait ValidationFunction: Send + Sync {
    async fn validate(&self, request: &StreamWeaveHttpRequestChunk) -> ValidationResult;
}

/// Validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Valid,
    Invalid { errors: Vec<CustomValidationError> },
}

/// Custom validation error for our use case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

impl CustomValidationError {
    pub fn new(field: &str, message: &str, code: &str) -> Self {
        Self { 
            field: field.to_string(), 
            message: message.to_string(), 
            code: code.to_string() 
        }
    }
}

/// JSON schema validator
pub struct JsonSchemaValidator {
    schema: serde_json::Value,
}

impl JsonSchemaValidator {
    pub fn new(schema: serde_json::Value) -> Self {
        Self { schema }
    }

    pub fn from_str(schema_str: &str) -> Result<Self, serde_json::Error> {
        let schema = serde_json::from_str(schema_str)?;
        Ok(Self::new(schema))
    }

    async fn validate_json(&self, body: &[u8]) -> ValidationResult {
        // Parse JSON
        let json_value: serde_json::Value = match serde_json::from_slice(body) {
            Ok(value) => value,
            Err(e) => {
                return ValidationResult::Invalid {
                    errors: vec![CustomValidationError::new(
                        "body",
                        &format!("Invalid JSON: {}", e),
                        "invalid_json",
                    )],
                };
            }
        };

        // In a real implementation, you would use a JSON schema validation library
        // like jsonschema or valico. For now, we'll do basic validation.
        self.validate_basic_json(&json_value)
    }

    fn validate_basic_json(&self, value: &serde_json::Value) -> ValidationResult {
        // Basic validation - check if required fields exist
        if let Some(_properties) = self.schema.get("properties") {
            if let Some(required) = self.schema.get("required") {
                if let Some(required_array) = required.as_array() {
                    for field in required_array {
                        if let Some(field_name) = field.as_str() {
                            if !value.get(field_name).is_some() {
                                return ValidationResult::Invalid {
                                    errors: vec![CustomValidationError::new(
                                        field_name,
                                        &format!("Field '{}' is required", field_name),
                                        "required",
                                    )],
                                };
                            }
                        }
                    }
                }
            }
        }

        ValidationResult::Valid
    }
}

/// Request validation transformer
pub struct RequestValidationTransformer {
    rules: Arc<Vec<ValidationRule>>,
    transformer_config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

impl RequestValidationTransformer {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(Vec::new()),
            transformer_config: TransformerConfig::default(),
        }
    }

    pub fn with_rule(mut self, rule: ValidationRule) -> Self {
        Arc::get_mut(&mut self.rules).unwrap().push(rule);
        self
    }

    pub fn with_rules(mut self, rules: Vec<ValidationRule>) -> Self {
        Arc::get_mut(&mut self.rules).unwrap().extend(rules);
        self
    }

    pub fn with_transformer_config(mut self, config: TransformerConfig<StreamWeaveHttpRequestChunk>) -> Self {
        self.transformer_config = config;
        self
    }

    #[allow(dead_code)]
    async fn validate_request(&self, request: &StreamWeaveHttpRequestChunk) -> ValidationResult {
        let mut all_errors = Vec::new();

        for rule in self.rules.iter() {
            match rule {
                ValidationRule::RequiredHeaders(required_headers) => {
                    for header_name in required_headers {
                        if !request.headers.contains_key(header_name) {
                            all_errors.push(CustomValidationError::new(
                                &format!("header.{}", header_name),
                                &format!("Header '{}' is required", header_name),
                                "required_header",
                            ));
                        }
                    }
                }
                ValidationRule::HeaderPattern(header_name, pattern) => {
                    if let Some(header_value) = request.headers.get(header_name) {
                        if let Ok(header_str) = header_value.to_str() {
                            let regex = match regex::Regex::new(&pattern) {
                                Ok(regex) => regex,
                                Err(_) => {
                                    all_errors.push(CustomValidationError::new(
                                &format!("header.{}", header_name),
                                "Invalid regex pattern",
                                "invalid_regex",
                                    ));
                                    continue;
                                }
                            };
                            
                            if !regex.is_match(header_str) {
                                all_errors.push(CustomValidationError::new(
                                &format!("header.{}", header_name),
                                &format!("Header '{}' does not match pattern '{}'", header_name, pattern),
                                "pattern_mismatch",
                                ));
                            }
                        } else {
                            all_errors.push(CustomValidationError::new(
                                &format!("header.{}", header_name),
                                &format!("Header '{}' contains invalid characters", header_name),
                                "invalid_header_value",
                            ));
                        }
                    }
                }
                ValidationRule::ContentType(expected_type) => {
                    if let Some(content_type) = request.headers.get("content-type") {
                        if let Ok(content_type_str) = content_type.to_str() {
                            if !content_type_str.starts_with(expected_type) {
                                all_errors.push(CustomValidationError::new(
                                "content-type",
                                &format!("Expected content type '{}', got '{}'", expected_type, content_type_str),
                                "invalid_content_type",
                                ));
                            }
                        }
                    } else {
                        all_errors.push(CustomValidationError::new(
                            "content-type",
                            "Content-Type header is required",
                            "required_header",
                        ));
                    }
                }
                ValidationRule::MaxBodySize(max_size) => {
                    if request.chunk.len() > *max_size {
                        all_errors.push(CustomValidationError::new(
                            "body",
                            &format!("Body size {} exceeds maximum allowed size {}", request.chunk.len(), max_size),
                            "body_too_large",
                        ));
                    }
                }
                ValidationRule::JsonSchema(schema_str) => {
                    let schema = match serde_json::from_str(&schema_str) {
                        Ok(schema) => schema,
                        Err(e) => {
                            all_errors.push(CustomValidationError::new(
                                "schema",
                                &format!("Invalid JSON schema: {}", e),
                                "invalid_schema",
                            ));
                            continue;
                        }
                    };
                    
                    let validator = JsonSchemaValidator::new(schema);
                    match validator.validate_json(&request.chunk).await {
                        ValidationResult::Valid => {}
                        ValidationResult::Invalid { errors } => {
                            all_errors.extend(errors);
                        }
                    }
                }
                ValidationRule::Custom(validator) => {
                    match validator.validate(request).await {
                        ValidationResult::Valid => {}
                        ValidationResult::Invalid { errors } => {
                            all_errors.extend(errors);
                        }
                    }
                }
            }
        }

        if all_errors.is_empty() {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid { errors: all_errors }
        }
    }
}

impl Default for RequestValidationTransformer {
    fn default() -> Self {
        Self::new()
    }
}

impl Input for RequestValidationTransformer {
    type Input = StreamWeaveHttpRequestChunk;
    type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for RequestValidationTransformer {
    type Output = StreamWeaveHttpRequestChunk;
    type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for RequestValidationTransformer {

    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
        let rules = Arc::clone(&self.rules);

        Box::pin(async_stream::stream! {
            let mut input_stream = input;
            
            while let Some(request) = input_stream.next().await {
                // Validate the request
                let validation_result = {
                    let mut all_errors = Vec::new();

                    for rule in rules.iter() {
                        match rule {
                            ValidationRule::RequiredHeaders(required_headers) => {
                                for header_name in required_headers {
                                    if !request.headers.contains_key(header_name) {
                                        all_errors.push(CustomValidationError::new(
                                            &format!("header.{}", header_name),
                                            &format!("Header '{}' is required", header_name),
                                            "required_header",
                                        ));
                                    }
                                }
                            }
                            ValidationRule::HeaderPattern(header_name, pattern) => {
                                if let Some(header_value) = request.headers.get(header_name) {
                                    if let Ok(header_str) = header_value.to_str() {
                                        let regex = match regex::Regex::new(&pattern) {
                                            Ok(regex) => regex,
                                            Err(_) => {
                                                all_errors.push(CustomValidationError::new(
                                                    &format!("header.{}", header_name),
                                                    "Invalid regex pattern",
                                                    "invalid_regex",
                                                ));
                                                continue;
                                            }
                                        };
                                        
                                        if !regex.is_match(header_str) {
                                            all_errors.push(CustomValidationError::new(
                                                &format!("header.{}", header_name),
                                                &format!("Header '{}' does not match pattern '{}'", header_name, pattern),
                                                "pattern_mismatch",
                                            ));
                                        }
                                    } else {
                                        all_errors.push(CustomValidationError::new(
                                            &format!("header.{}", header_name),
                                            &format!("Header '{}' contains invalid characters", header_name),
                                            "invalid_header_value",
                                        ));
                                    }
                                }
                            }
                            ValidationRule::ContentType(expected_type) => {
                                if let Some(content_type) = request.headers.get("content-type") {
                                    if let Ok(content_type_str) = content_type.to_str() {
                                        if !content_type_str.starts_with(expected_type) {
                                            all_errors.push(CustomValidationError::new(
                                                "content-type",
                                                &format!("Expected content type '{}', got '{}'", expected_type, content_type_str),
                                                "invalid_content_type",
                                            ));
                                        }
                                    }
                                } else {
                                    all_errors.push(CustomValidationError::new(
                                        "content-type",
                                        "Content-Type header is required",
                                        "required_header",
                                    ));
                                }
                            }
                            ValidationRule::MaxBodySize(max_size) => {
                                if request.chunk.len() > *max_size {
                                    all_errors.push(CustomValidationError::new(
                                        "body",
                                        &format!("Body size {} exceeds maximum allowed size {}", request.chunk.len(), max_size),
                                        "body_too_large",
                                    ));
                                }
                            }
                            ValidationRule::JsonSchema(schema_str) => {
                                let schema = match serde_json::from_str(&schema_str) {
                                    Ok(schema) => schema,
                                    Err(e) => {
                                        all_errors.push(CustomValidationError::new(
                                            "schema",
                                            &format!("Invalid JSON schema: {}", e),
                                            "invalid_schema",
                                        ));
                                        continue;
                                    }
                                };
                                
                                let validator = JsonSchemaValidator::new(schema);
                                match validator.validate_json(&request.chunk).await {
                                    ValidationResult::Valid => {}
                                    ValidationResult::Invalid { errors } => {
                                        all_errors.extend(errors);
                                    }
                                }
                            }
                            ValidationRule::Custom(validator) => {
                                match validator.validate(&request).await {
                                    ValidationResult::Valid => {}
                                    ValidationResult::Invalid { errors } => {
                                        all_errors.extend(errors);
                                    }
                                }
                            }
                        }
                    }

                    if all_errors.is_empty() {
                        ValidationResult::Valid
                    } else {
                        ValidationResult::Invalid { errors: all_errors }
                    }
                };

                match validation_result {
                    ValidationResult::Valid => {
                        // Validation passed, continue processing
                        yield request;
                    }
                    ValidationResult::Invalid { .. } => {
                        // Validation failed, skip this request
                        continue;
                    }
                }
            }
        })
    }

    fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
        self.transformer_config = config;
    }

    fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
        &self.transformer_config
    }

    fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
        &mut self.transformer_config
    }
}


/// Response validation transformer
pub struct ResponseValidationTransformer {
    rules: Arc<Vec<ValidationRule>>,
    transformer_config: TransformerConfig<StreamWeaveHttpResponse>,
}

impl ResponseValidationTransformer {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(Vec::new()),
            transformer_config: TransformerConfig::default(),
        }
    }

    pub fn with_rule(mut self, rule: ValidationRule) -> Self {
        Arc::get_mut(&mut self.rules).unwrap().push(rule);
        self
    }

    pub fn with_rules(mut self, rules: Vec<ValidationRule>) -> Self {
        Arc::get_mut(&mut self.rules).unwrap().extend(rules);
        self
    }

    pub fn with_transformer_config(mut self, config: TransformerConfig<StreamWeaveHttpResponse>) -> Self {
        self.transformer_config = config;
        self
    }

    #[allow(dead_code)]
    async fn validate_response(&self, response: &StreamWeaveHttpResponse) -> ValidationResult {
        let mut all_errors = Vec::new();

        for rule in self.rules.iter() {
            match rule {
                ValidationRule::RequiredHeaders(required_headers) => {
                    for header_name in required_headers {
                        if !response.headers.contains_key(header_name) {
                            all_errors.push(CustomValidationError::new(
                                &format!("header.{}", header_name),
                                &format!("Header '{}' is required", header_name),
                                "required_header",
                            ));
                        }
                    }
                }
                ValidationRule::HeaderPattern(header_name, pattern) => {
                    if let Some(header_value) = response.headers.get(header_name) {
                        if let Ok(header_str) = header_value.to_str() {
                            let regex = match regex::Regex::new(&pattern) {
                                Ok(regex) => regex,
                                Err(_) => {
                                    all_errors.push(CustomValidationError::new(
                                &format!("header.{}", header_name),
                                "Invalid regex pattern",
                                "invalid_regex",
                                    ));
                                    continue;
                                }
                            };
                            
                            if !regex.is_match(header_str) {
                                all_errors.push(CustomValidationError::new(
                                &format!("header.{}", header_name),
                                &format!("Header '{}' does not match pattern '{}'", header_name, pattern),
                                "pattern_mismatch",
                                ));
                            }
                        }
                    }
                }
                ValidationRule::ContentType(expected_type) => {
                    if let Some(content_type) = response.headers.get("content-type") {
                        if let Ok(content_type_str) = content_type.to_str() {
                            if !content_type_str.starts_with(expected_type) {
                                all_errors.push(CustomValidationError::new(
                                "content-type",
                                &format!("Expected content type '{}', got '{}'", expected_type, content_type_str),
                                "invalid_content_type",
                                ));
                            }
                        }
                    }
                }
                ValidationRule::MaxBodySize(max_size) => {
                    if response.body.len() > *max_size {
                        all_errors.push(CustomValidationError::new(
                            "body",
                            &format!("Body size {} exceeds maximum allowed size {}", response.body.len(), max_size),
                            "body_too_large",
                        ));
                    }
                }
                ValidationRule::JsonSchema(schema_str) => {
                    let schema = match serde_json::from_str(&schema_str) {
                        Ok(schema) => schema,
                        Err(e) => {
                            all_errors.push(CustomValidationError::new(
                                "schema",
                                &format!("Invalid JSON schema: {}", e),
                                "invalid_schema",
                            ));
                            continue;
                        }
                    };
                    
                    let validator = JsonSchemaValidator::new(schema);
                    match validator.validate_json(&response.body).await {
                        ValidationResult::Valid => {}
                        ValidationResult::Invalid { errors } => {
                            all_errors.extend(errors);
                        }
                    }
                }
                ValidationRule::Custom(_) => {
                    // Custom validation not supported for responses in this implementation
                }
            }
        }

        if all_errors.is_empty() {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid { errors: all_errors }
        }
    }
}

impl Default for ResponseValidationTransformer {
    fn default() -> Self {
        Self::new()
    }
}

impl Input for ResponseValidationTransformer {
    type Input = StreamWeaveHttpResponse;
    type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for ResponseValidationTransformer {
    type Output = StreamWeaveHttpResponse;
    type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for ResponseValidationTransformer {

    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
        let rules = Arc::clone(&self.rules);

        Box::pin(async_stream::stream! {
            let mut input_stream = input;
            
            while let Some(response) = input_stream.next().await {
                // Validate the response
                let validation_result = {
                    let mut all_errors = Vec::new();

                    for rule in rules.iter() {
                        match rule {
                            ValidationRule::RequiredHeaders(required_headers) => {
                                for header_name in required_headers {
                                    if !response.headers.contains_key(header_name) {
                                        all_errors.push(CustomValidationError::new(
                                            &format!("header.{}", header_name),
                                            &format!("Header '{}' is required", header_name),
                                            "required_header",
                                        ));
                                    }
                                }
                            }
                            ValidationRule::HeaderPattern(header_name, pattern) => {
                                if let Some(header_value) = response.headers.get(header_name) {
                                    if let Ok(header_str) = header_value.to_str() {
                                        let regex = match regex::Regex::new(&pattern) {
                                            Ok(regex) => regex,
                                            Err(_) => {
                                                all_errors.push(CustomValidationError::new(
                                                    &format!("header.{}", header_name),
                                                    "Invalid regex pattern",
                                                    "invalid_regex",
                                                ));
                                                continue;
                                            }
                                        };
                                        
                                        if !regex.is_match(header_str) {
                                            all_errors.push(CustomValidationError::new(
                                                &format!("header.{}", header_name),
                                                &format!("Header '{}' does not match pattern '{}'", header_name, pattern),
                                                "pattern_mismatch",
                                            ));
                                        }
                                    }
                                }
                            }
                            ValidationRule::ContentType(expected_type) => {
                                if let Some(content_type) = response.headers.get("content-type") {
                                    if let Ok(content_type_str) = content_type.to_str() {
                                        if !content_type_str.starts_with(expected_type) {
                                            all_errors.push(CustomValidationError::new(
                                                "content-type",
                                                &format!("Expected content type '{}', got '{}'", expected_type, content_type_str),
                                                "invalid_content_type",
                                            ));
                                        }
                                    }
                                }
                            }
                            ValidationRule::MaxBodySize(max_size) => {
                                if response.body.len() > *max_size {
                                    all_errors.push(CustomValidationError::new(
                                        "body",
                                        &format!("Body size {} exceeds maximum allowed size {}", response.body.len(), max_size),
                                        "body_too_large",
                                    ));
                                }
                            }
                            ValidationRule::JsonSchema(schema_str) => {
                                let schema = match serde_json::from_str(&schema_str) {
                                    Ok(schema) => schema,
                                    Err(e) => {
                                        all_errors.push(CustomValidationError::new(
                                            "schema",
                                            &format!("Invalid JSON schema: {}", e),
                                            "invalid_schema",
                                        ));
                                        continue;
                                    }
                                };
                                
                                let validator = JsonSchemaValidator::new(schema);
                                match validator.validate_json(&response.body).await {
                                    ValidationResult::Valid => {}
                                    ValidationResult::Invalid { errors } => {
                                        all_errors.extend(errors);
                                    }
                                }
                            }
                            ValidationRule::Custom(_) => {
                                // Custom validation not supported for responses
                            }
                        }
                    }

                    if all_errors.is_empty() {
                        ValidationResult::Valid
                    } else {
                        ValidationResult::Invalid { errors: all_errors }
                    }
                };

                match validation_result {
                    ValidationResult::Valid => {
                        // Validation passed, continue processing
                        yield response;
                    }
                    ValidationResult::Invalid { .. } => {
                        // Validation failed, skip this response
                        continue;
                    }
                }
            }
        })
    }

    fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
        self.transformer_config = config;
    }

    fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
        &self.transformer_config
    }

    fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
        &mut self.transformer_config
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{
        http_request_chunk::StreamWeaveHttpRequestChunk,
        connection_info::ConnectionInfo,
    };
    use http::{Method, Uri, Version};
    use http::HeaderMap;

    fn create_test_request(headers: HeaderMap, body: &[u8]) -> StreamWeaveHttpRequestChunk {
        let connection_info = ConnectionInfo::new(
            "127.0.0.1:8080".parse().unwrap(),
            "0.0.0.0:3000".parse().unwrap(),
            Version::HTTP_11,
        );
        
        StreamWeaveHttpRequestChunk::new(
            Method::POST,
            Uri::from_static("/test"),
            headers,
            body.to_vec().into(),
            connection_info,
            true,
        )
    }

    #[test]
    fn test_validation_error_creation() {
        let error = CustomValidationError::new(
            "field",
            "Error message",
            "error_code",
        );
        
        assert_eq!(error.field, "field");
        assert_eq!(error.message, "Error message");
        assert_eq!(error.code, "error_code");
    }

    #[test]
    fn test_json_schema_validator_creation() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });
        
        let validator = JsonSchemaValidator::new(schema);
        // Just test that it can be created
        assert!(validator.schema.get("type").is_some());
    }

    #[test]
    fn test_json_schema_validator_from_str() {
        let schema_str = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
        let validator = JsonSchemaValidator::from_str(schema_str);
        assert!(validator.is_ok());
    }

    #[tokio::test]
    async fn test_json_schema_validation_valid() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });
        
        let validator = JsonSchemaValidator::new(schema);
        let valid_json = r#"{"name": "test"}"#;
        
        let result = validator.validate_json(valid_json.as_bytes()).await;
        match result {
            ValidationResult::Valid => {}
            ValidationResult::Invalid { errors } => {
                panic!("Expected valid result, got errors: {:?}", errors);
            }
        }
    }

    #[tokio::test]
    async fn test_json_schema_validation_invalid() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });
        
        let validator = JsonSchemaValidator::new(schema);
        let invalid_json = r#"{"age": 25}"#; // Missing required "name" field
        
        let result = validator.validate_json(invalid_json.as_bytes()).await;
        match result {
            ValidationResult::Valid => {
                panic!("Expected invalid result");
            }
            ValidationResult::Invalid { errors } => {
                assert!(!errors.is_empty());
                assert!(errors.iter().any(|e| e.field == "name"));
            }
        }
    }

    #[tokio::test]
    async fn test_request_validation_transformer_required_headers() {
        let transformer = RequestValidationTransformer::new()
            .with_rule(ValidationRule::RequiredHeaders(vec!["authorization".to_string()]));
        
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer token".parse().unwrap());
        let request = create_test_request(headers, b"test body");
        
        let result = transformer.validate_request(&request).await;
        match result {
            ValidationResult::Valid => {}
            ValidationResult::Invalid { errors } => {
                panic!("Expected valid result, got errors: {:?}", errors);
            }
        }
    }

    #[tokio::test]
    async fn test_request_validation_transformer_missing_required_headers() {
        let transformer = RequestValidationTransformer::new()
            .with_rule(ValidationRule::RequiredHeaders(vec!["authorization".to_string()]));
        
        let headers = HeaderMap::new();
        let request = create_test_request(headers, b"test body");
        
        let result = transformer.validate_request(&request).await;
        match result {
            ValidationResult::Valid => {
                panic!("Expected invalid result");
            }
            ValidationResult::Invalid { errors } => {
                assert!(!errors.is_empty());
                assert!(errors.iter().any(|e| e.field == "header.authorization"));
            }
        }
    }

    #[tokio::test]
    async fn test_request_validation_transformer_content_type() {
        let transformer = RequestValidationTransformer::new()
            .with_rule(ValidationRule::ContentType("application/json".to_string()));
        
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        let request = create_test_request(headers, b"{}");
        
        let result = transformer.validate_request(&request).await;
        match result {
            ValidationResult::Valid => {}
            ValidationResult::Invalid { errors } => {
                panic!("Expected valid result, got errors: {:?}", errors);
            }
        }
    }

    #[tokio::test]
    async fn test_request_validation_transformer_max_body_size() {
        let transformer = RequestValidationTransformer::new()
            .with_rule(ValidationRule::MaxBodySize(100));
        
        let headers = HeaderMap::new();
        let request = create_test_request(headers, &vec![0u8; 50]); // 50 bytes
        
        let result = transformer.validate_request(&request).await;
        match result {
            ValidationResult::Valid => {}
            ValidationResult::Invalid { errors } => {
                panic!("Expected valid result, got errors: {:?}", errors);
            }
        }
    }

    #[tokio::test]
    async fn test_request_validation_transformer_body_too_large() {
        let transformer = RequestValidationTransformer::new()
            .with_rule(ValidationRule::MaxBodySize(100));
        
        let headers = HeaderMap::new();
        let request = create_test_request(headers, &vec![0u8; 200]); // 200 bytes
        
        let result = transformer.validate_request(&request).await;
        match result {
            ValidationResult::Valid => {
                panic!("Expected invalid result");
            }
            ValidationResult::Invalid { errors } => {
                assert!(!errors.is_empty());
                assert!(errors.iter().any(|e| e.field == "body"));
            }
        }
    }

    #[test]
    fn test_validation_transformer_creation() {
        let transformer = RequestValidationTransformer::new()
            .with_rule(ValidationRule::RequiredHeaders(vec!["authorization".to_string()]))
            .with_rule(ValidationRule::MaxBodySize(1024));
        
        assert_eq!(transformer.rules.len(), 2);
    }
}
