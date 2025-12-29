# streamweave-window

[![Crates.io](https://img.shields.io/crates/v/streamweave-window.svg)](https://crates.io/crates/streamweave-window)
[![Documentation](https://docs.rs/streamweave-window/badge.svg)](https://docs.rs/streamweave-window)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Windowing operations for StreamWeave**  
*Group stream elements into bounded windows for aggregation and processing.*

The `streamweave-window` package provides windowing operations for StreamWeave streams. It supports time-based windows (tumbling, sliding), count-based windows, session windows, and configurable window triggers and late data policies.

## ✨ Key Features

- **Window Types**: Tumbling, Sliding, Session windows
- **Time-Based Windows**: Event time and processing time support
- **Count-Based Windows**: Window by number of elements
- **Window Assigners**: Assign elements to windows
- **Window Triggers**: Control when windows fire
- **Late Data Policy**: Handle late-arriving elements
- **Window Transformers**: Ready-to-use window transformers

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-window = "0.4.0"
```

## 🚀 Quick Start

### Tumbling Window

```rust
use streamweave_window::{TumblingWindowAssigner, WindowConfig};
use std::time::Duration;

// Create a tumbling window of 5 seconds
let assigner = TumblingWindowAssigner::new(Duration::from_secs(5));
let config = WindowConfig::default();

// Use with window transformer
let transformer = WindowTransformer::new(assigner, config);
```

### Sliding Window

```rust
use streamweave_window::SlidingWindowAssigner;

// Create a sliding window: 10 second size, 5 second slide
let assigner = SlidingWindowAssigner::new(
    Duration::from_secs(10),
    Duration::from_secs(5)
);
```

## 📖 API Overview

### Window Types

**TimeWindow:**
- Represents a window with start and end timestamps
- Used for time-based windowing

**Window Assigners:**
- `TumblingWindowAssigner` - Fixed-size, non-overlapping windows
- `SlidingWindowAssigner` - Fixed-size, overlapping windows
- `SessionWindowAssigner` - Gap-based dynamic windows

### Window Configuration

```rust
pub struct WindowConfig {
    pub late_data_policy: LateDataPolicy,
    pub allowed_lateness: Option<Duration>,
    // ... other config options
}
```

### Window Triggers

Control when windows fire:

```rust
pub enum TriggerResult {
    Continue,      // Keep accumulating
    Fire,          // Emit results, keep state
    FireAndPurge,  // Emit results, clear state
    Purge,         // Discard state
}
```

### Late Data Policy

Handle late-arriving elements:

```rust
pub enum LateDataPolicy {
    Drop,                    // Drop late elements
    SideOutput,              // Emit to side output
    AllowLateness(Duration), // Include in window
}
```

## 📚 Usage Examples

### Tumbling Window

Fixed-size, non-overlapping windows:

```rust
use streamweave_window::{TumblingWindowAssigner, WindowConfig};
use std::time::Duration;

let assigner = TumblingWindowAssigner::new(Duration::from_secs(5));
let config = WindowConfig::default();

// Elements are grouped into 5-second windows
// [0-5s), [5-10s), [10-15s), ...
```

### Sliding Window

Fixed-size, overlapping windows:

```rust
use streamweave_window::SlidingWindowAssigner;

// 10 second window, 5 second slide
let assigner = SlidingWindowAssigner::new(
    Duration::from_secs(10),
    Duration::from_secs(5)
);

// Windows: [0-10s), [5-15s), [10-20s), ...
```

### Session Window

Gap-based dynamic windows:

```rust
use streamweave_window::SessionWindowAssigner;

// Session gap of 30 seconds
let assigner = SessionWindowAssigner::new(Duration::from_secs(30));

// Windows are created dynamically based on activity gaps
```

### Count-Based Windows

Window by number of elements:

```rust
use streamweave_window::CountWindowAssigner;

// Window of 100 elements
let assigner = CountWindowAssigner::new(100);
```

### Window Transformers

Use ready-to-use window transformers:

```rust
use streamweave_window::transformers::WindowTransformer;

let assigner = TumblingWindowAssigner::new(Duration::from_secs(5));
let transformer = WindowTransformer::new(assigner, WindowConfig::default());

// Use in pipeline
pipeline.transformer(transformer);
```

### Late Data Handling

Configure late data policy:

```rust
use streamweave_window::{WindowConfig, LateDataPolicy};
use std::time::Duration;

// Allow lateness up to 1 minute
let config = WindowConfig {
    late_data_policy: LateDataPolicy::AllowLateness(Duration::from_secs(60)),
    ..Default::default()
};

// Or drop late data
let config = WindowConfig {
    late_data_policy: LateDataPolicy::Drop,
    ..Default::default()
};

// Or emit to side output
let config = WindowConfig {
    late_data_policy: LateDataPolicy::SideOutput,
    ..Default::default()
};
```

### Time-Based Windows

Use event time or processing time:

```rust
use streamweave_window::{TimeWindow, DateTime, Utc};

// Create a time window
let window = TimeWindow::new(
    Utc::now(),
    Utc::now() + Duration::from_secs(10)
);

// Check if timestamp is in window
let timestamp = Utc::now();
if window.contains(timestamp) {
    // Process element
}
```

## 🏗️ Architecture

Windows group elements for bounded processing:

```
┌─────────────┐
│   Stream    │───elements───>┌──────────────┐
└─────────────┘               │ WindowAssigner│
                              │              │
                              │  ┌────────┐ │
                              │  │ Window │ │
                              │  └────────┘ │
                              └──────────────┘
                                     │
                                     ▼
                              ┌──────────────┐
                              │ Aggregation  │
                              └──────────────┘
```

**Window Flow:**
1. Elements arrive in stream
2. Window assigner assigns elements to windows
3. Elements accumulate in windows
4. Window trigger fires window
5. Aggregation processes window contents
6. Results emitted

## 🔧 Configuration

### Window Assigners

**Tumbling:**
- Fixed-size, non-overlapping
- Simple and efficient
- Best for regular aggregations

**Sliding:**
- Fixed-size, overlapping
- More windows, more computation
- Best for smooth aggregations

**Session:**
- Gap-based, dynamic size
- Adapts to data patterns
- Best for user sessions

### Late Data Policies

**Drop:**
- Discard late elements
- Simple, no overhead
- Best for real-time processing

**SideOutput:**
- Emit to separate stream
- Enables separate processing
- Best for analysis

**AllowLateness:**
- Include in window
- Refire if needed
- Best for accuracy

## 🔍 Error Handling

Window operations return `WindowResult<T>`:

```rust
pub enum WindowError {
    InvalidConfig(String),
    NotFound(String),
    WindowClosed(String),
    StateError(String),
}
```

## ⚡ Performance Considerations

- **Window State**: Windows maintain state for elements
- **Memory Usage**: More windows = more memory
- **Trigger Frequency**: Frequent triggers = more computation
- **Late Data**: Late data handling adds overhead

## 📝 Examples

For more examples, see:
- [Windowing Example](https://github.com/Industrial/streamweave/tree/main/examples/windowing)
- [Time-Based Windows](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-window` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `futures` - Stream utilities
- `chrono` - Timestamp support
- `async-stream` - Stream generation

## 🎯 Use Cases

Windowing is used for:

1. **Time-Based Aggregations**: Sum, average, count over time
2. **Sliding Aggregations**: Moving averages, trends
3. **Session Analysis**: User session tracking
4. **Bounded Processing**: Process unbounded streams in bounded windows
5. **Late Data Handling**: Handle out-of-order data

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-window)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/window)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API
- [streamweave-graph](../graph/README.md) - Graph API
- [streamweave-stateful](../stateful/README.md) - Stateful processing

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

