# effect-stream-time

A time-based stream implementation for [effect-stream](https://github.com/yourusername/effect-stream).

## Overview

This crate provides a time-based stream implementation that emits values at regular intervals. It's built on top of `effect-stream` and provides a simple way to create streams that emit values based on time.

## Usage

```rust
use effect_stream_time::{TimeStream, TimeStreamError};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), TimeStreamError> {
    // Create a stream that emits a value every second
    let stream = TimeStream::new(Duration::from_secs(1));
    
    // Convert it to an EffectStream
    let mut effect_stream = stream.into_effect_stream();
    
    // Read values from the stream
    while let Some(value) = effect_stream.next().await? {
        println!("Received value: {:?}", value);
    }
    
    Ok(())
}
```

## Features

- Emit values at regular time intervals
- Thread-safe and async-friendly
- Built on top of `effect-stream`
- Zero dependencies (except for `effect-stream` and `tokio`)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 