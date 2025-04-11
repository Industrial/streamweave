# effect-stream-env

Environment variable streams for `effect-stream`. This crate provides streams for reading and monitoring environment variables.

## Usage

```rust
use effect_stream_env::EnvStream;

// Create a stream that emits environment variable changes
let env_stream = EnvStream::new("MY_VAR");
```

## Features

- Environment variable monitoring
- Change detection
- Thread-safe and async-friendly
- Zero dependencies except for `effect-stream` and `tokio`

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 