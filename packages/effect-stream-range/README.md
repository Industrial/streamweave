# effect-stream-range

Range-based streams for `effect-stream`. This crate provides streams for generating sequences of numbers and other ranges.

## Usage

```rust
use effect_stream_range::RangeStream;

// Create a stream that emits numbers from 1 to 10
let range_stream = RangeStream::new(1..=10);
```

## Features

- Numeric range streams
- Custom step sizes
- Reverse ranges
- Thread-safe and async-friendly
- Zero dependencies except for `effect-stream` and `tokio`

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 