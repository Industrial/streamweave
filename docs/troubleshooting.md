# Troubleshooting

Common issues and solutions when using StreamWeave.

## Build Issues

### Missing Dependencies

If you encounter build errors related to native dependencies (e.g., Kafka, Redis):

1. Ensure you have the required system libraries installed
2. Use `devenv.sh` which includes all necessary dependencies
3. Check that feature flags are correctly enabled in `Cargo.toml`

### WASM Build Failures

If WASM builds fail:

1. Ensure you're using the `wasm` feature flag
2. Check that you're not using native-only features in WASM code
3. Verify `wasm32-unknown-unknown` target is installed: `rustup target add wasm32-unknown-unknown`

## Runtime Issues

### Pipeline Not Producing Output

- Check that the producer is actually generating items
- Verify error handling strategy isn't silently skipping all items
- Add logging to debug the pipeline flow

### Memory Issues with Large Streams

- Use streaming consumers instead of collecting everything
- Enable backpressure handling
- Use batching to reduce memory pressure

### Performance Issues

- Profile your pipeline to identify bottlenecks
- Consider using `RateLimitTransformer` to control throughput
- Use `BatchTransformer` to reduce per-item overhead
- Enable release mode optimizations

## Documentation Issues

### Missing Documentation

If documentation is incomplete:

1. Run `cargo doc --all-features` to generate docs
2. Check that public APIs have doc comments
3. Review the documentation standards in `.cursor/rules/documentation_standards.mdc`

### Documentation Not Updating

- Clear `target/doc` directory and regenerate
- Ensure you're using the correct feature flags
- Check that `bin/docs` script is working correctly

## Integration Issues

### Kafka Connection Problems

- Verify Kafka is running and accessible
- Check broker addresses and ports
- Review `docs/KAFKA_SETUP.md` for setup instructions
- Use `devenv.sh` for automatic Kafka setup

### Database Connection Issues

- Verify database credentials and connection strings
- Check that the database feature flag is enabled
- Ensure connection pooling is configured correctly

### HTTP Server Issues

- Verify Axum dependencies are available
- Check that `http-server` feature is enabled
- Ensure routes are correctly configured

## Getting Help

- Check the [Examples](../examples/) directory
- Review the [API Documentation](../target/doc/streamweave/index.html)
- Open an issue on GitHub with details about your problem

