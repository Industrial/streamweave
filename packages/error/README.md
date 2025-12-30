# streamweave-error

[![Crates.io](https://img.shields.io/crates/v/streamweave-error.svg)](https://crates.io/crates/streamweave-error)
[![Documentation](https://docs.rs/streamweave-error/badge.svg)](https://docs.rs/streamweave-error)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Error handling system for StreamWeave**  
*Comprehensive error handling with multiple strategies and rich error context.*

The `streamweave-error` package provides a comprehensive error handling system for StreamWeave pipelines and graphs. It includes error strategies, error actions, rich error context, and component information for effective error handling and debugging.

## ✨ Key Features

- **ErrorStrategy Enum**: Multiple error handling strategies (Stop, Skip, Retry, Custom)
- **ErrorAction Enum**: Actions to take when errors occur (Stop, Skip, Retry)
- **StreamError Type**: Rich error type with context and component information
- **ErrorContext**: Detailed context about when and where errors occurred
- **ComponentInfo**: Component identification for error reporting
- **PipelineError**: Pipeline-specific error wrapper

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-error = "0.3.0"
```

## 🚀 Quick Start

### Basic Error Handling

```rust
use streamweave_error::{ErrorStrategy, ErrorAction, StreamError, ErrorContext, ComponentInfo};

// Configure error strategy
let strategy = ErrorStrategy::Skip;  // Skip errors and continue

// Create error context
let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "MyTransformer".to_string(),
    component_type: "Transformer".to_string(),
};

// Create component info
let component = ComponentInfo::new(
    "my-transformer".to_string(),
    "MyTransformer".to_string(),
);

// Create stream error
let error = StreamError::new(
    Box::new(std::io::Error::other("Something went wrong")),
    context,
    component,
);

// Determine action based on strategy
let action = match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
    _ => ErrorAction::Stop,
};
```

## 📖 API Overview

### ErrorStrategy

The `ErrorStrategy` enum defines how components should handle errors:

```rust
pub enum ErrorStrategy<T> {
    Stop,                    // Stop processing on error
    Skip,                    // Skip errors and continue
    Retry(usize),            // Retry up to N times
    Custom(CustomHandler),   // Custom error handling logic
}
```

**Stop Strategy:**
- Default behavior
- Stops processing immediately when an error occurs
- Ensures data integrity by preventing partial results

**Skip Strategy:**
- Skips items that cause errors
- Continues processing remaining items
- Useful for data cleaning scenarios

**Retry Strategy:**
- Retries failed operations up to specified number of times
- Useful for transient failures (network timeouts, etc.)
- Stops after retry limit is exhausted

**Custom Strategy:**
- Allows fine-grained control over error handling
- Can base decisions on error type, context, or retry count
- Enables complex error handling logic

### ErrorAction

The `ErrorAction` enum represents the action to take when an error occurs:

```rust
pub enum ErrorAction {
    Stop,   // Stop processing immediately
    Skip,   // Skip the item and continue
    Retry,  // Retry the operation
}
```

### StreamError

The `StreamError` type provides rich error information:

```rust
pub struct StreamError<T> {
    pub source: Box<dyn Error + Send + Sync>,  // Original error
    pub context: ErrorContext<T>,                // Error context
    pub component: ComponentInfo,                 // Component info
    pub retries: usize,                          // Retry count
}
```

### ErrorContext

The `ErrorContext` struct provides detailed information about when and where an error occurred:

```rust
pub struct ErrorContext<T> {
    pub timestamp: chrono::DateTime<chrono::Utc>,  // When error occurred
    pub item: Option<T>,                            // Item being processed
    pub component_name: String,                      // Component name
    pub component_type: String,                      // Component type
}
```

### ComponentInfo

The `ComponentInfo` struct identifies the component that encountered an error:

```rust
pub struct ComponentInfo {
    pub name: String,       // Component name
    pub type_name: String,  // Component type name
}
```

## 📚 Usage Examples

### Stop Strategy

Stop processing on first error (default behavior):

```rust
use streamweave_error::ErrorStrategy;

let strategy = ErrorStrategy::<i32>::Stop;

// When an error occurs, processing stops immediately
// This ensures data integrity
```

### Skip Strategy

Skip errors and continue processing:

```rust
use streamweave_error::ErrorStrategy;

let strategy = ErrorStrategy::<i32>::Skip;

// Invalid items are skipped, processing continues
// Useful for data cleaning pipelines
```

### Retry Strategy

Retry failed operations:

```rust
use streamweave_error::ErrorStrategy;

let strategy = ErrorStrategy::<i32>::Retry(3);

// Retries up to 3 times before giving up
// Useful for transient failures like network timeouts
```

### Custom Error Handler

Implement custom error handling logic:

```rust
use streamweave_error::{ErrorStrategy, ErrorAction, StreamError};

let strategy = ErrorStrategy::<i32>::new_custom(|error: &StreamError<i32>| {
    // Retry transient errors
    if error.retries < 3 && is_transient(&error.source) {
        ErrorAction::Retry
    }
    // Skip validation errors
    else if is_validation_error(&error.source) {
        ErrorAction::Skip
    }
    // Stop on critical errors
    else {
        ErrorAction::Stop
    }
});
```

### Creating Error Context

Create detailed error context for debugging:

```rust
use streamweave_error::{ErrorContext, ComponentInfo, StreamError};

// Create error context
let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(problematic_item),
    component_name: "DataValidator".to_string(),
    component_type: "Transformer".to_string(),
};

// Create component info
let component = ComponentInfo::new(
    "validator".to_string(),
    "DataValidator".to_string(),
);

// Create stream error
let error = StreamError::new(
    Box::new(validation_error),
    context,
    component,
);
```

### Error Propagation Patterns

Handle errors at different levels:

```rust
use streamweave_error::{ErrorStrategy, StreamError, ErrorAction};

// Component-level error handling
fn handle_component_error(error: &StreamError<i32>) -> ErrorAction {
    match error.retries {
        0..=2 => ErrorAction::Retry,  // Retry first 3 attempts
        _ => ErrorAction::Skip,        // Skip after retries exhausted
    }
}

// Pipeline-level error handling
fn handle_pipeline_error(error: &StreamError<i32>) -> ErrorAction {
    if is_critical_error(&error.source) {
        ErrorAction::Stop  // Stop on critical errors
    } else {
        ErrorAction::Skip  // Skip non-critical errors
    }
}
```

## 🔧 Configuration

Error strategies can be configured at multiple levels:

### Component Level

```rust
use streamweave_error::ErrorStrategy;

// Configure producer error handling
let producer = MyProducer::new()
    .with_config(
        ProducerConfig::default()
            .with_error_strategy(ErrorStrategy::Retry(3))
    );

// Configure transformer error handling
let transformer = MyTransformer::new()
    .with_config(
        TransformerConfig::default()
            .with_error_strategy(ErrorStrategy::Skip)
    );

// Configure consumer error handling
let consumer = MyConsumer::new()
    .with_config(
        ConsumerConfig {
            error_strategy: ErrorStrategy::Stop,
            name: "output".to_string(),
        }
    );
```

### Pipeline Level

```rust
use streamweave::PipelineBuilder;
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer)
    .with_error_strategy(ErrorStrategy::Skip);  // Pipeline-wide default
```

## 🏗️ Architecture

The error handling system integrates at multiple levels:

```text
┌─────────────────┐
│   Component     │───encounters error───> StreamError
└─────────────────┘                           │
                                               │
                                               ▼
┌─────────────────┐                    ┌──────────────┐
│ ErrorStrategy   │───determines───>   │ ErrorAction  │
└─────────────────┘                    └──────────────┘
         │                                       │
         │                                       ▼
         │                              ┌──────────────┐
         └───uses context from───>       │ ErrorContext │
                                        └──────────────┘
```

## 🔍 Error Handling

### Error Strategies

**Stop (Default):**
- Ensures data integrity
- Prevents partial results
- Best for critical data processing

**Skip:**
- Allows partial results
- Continues processing
- Best for data cleaning and filtering

**Retry:**
- Handles transient failures
- Configurable retry count
- Best for network operations and external services

**Custom:**
- Maximum flexibility
- Can implement complex logic
- Best for domain-specific error handling

### Error Context

Error context provides:
- **Timestamp**: When the error occurred
- **Item**: The item being processed (if available)
- **Component Name**: Human-readable component identifier
- **Component Type**: Type name for debugging

### Component Information

Component info enables:
- **Logging**: Identify which component failed
- **Metrics**: Track errors per component
- **Debugging**: Locate source of errors
- **Monitoring**: Alert on component failures

## ⚡ Performance Considerations

- **Zero-Cost Abstractions**: Error strategies compile to efficient code
- **Minimal Overhead**: Error context creation is lightweight
- **Clone Efficiency**: Error types are designed for efficient cloning
- **Send + Sync**: All error types are thread-safe

## 📝 Examples

For more examples, see:
- [Error Handling Examples](https://github.com/Industrial/streamweave/tree/main/examples/error_handling)
- [Pipeline Examples](https://github.com/Industrial/streamweave/tree/main/examples/basic_pipeline)

## 🔗 Dependencies

`streamweave-error` depends on:

- `thiserror` - Error trait implementation
- `serde` - Serialization support
- `serde_json` - JSON serialization
- `chrono` - Timestamp support

## 🎯 Use Cases

The error handling system is used for:

1. **Data Validation**: Skip invalid records while processing valid ones
2. **Network Operations**: Retry transient network failures
3. **External Services**: Handle service unavailability gracefully
4. **Data Cleaning**: Skip malformed data and continue processing
5. **Critical Processing**: Stop on errors to ensure data integrity

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-error)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/error)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits that use error handling
- [streamweave-pipeline](../pipeline/README.md) - Pipeline error handling
- [streamweave-graph](../graph/README.md) - Graph error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

