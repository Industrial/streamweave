# effect-stream-data

Data transformation streams for `effect-stream`. This crate provides streams for transforming and processing data, including JSON parsing, data validation, and format conversion.

## Usage

```rust
use effect_stream_data::JsonStream;
use serde_json::Value;

// Create a stream that parses JSON strings into Value objects
let json_stream = JsonStream::new(input_stream);
```

## Features

- JSON parsing and serialization
- Data validation
- Format conversion
- Thread-safe and async-friendly
- Zero dependencies except for `effect-stream`, `tokio`, and `serde`

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 