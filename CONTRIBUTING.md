# Contributing to StreamWeave

Thank you for your interest in contributing to StreamWeave! This guide will help you get started.

## Development Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/Industrial/streamweave.git
   cd streamweave
   ```

2. **Set up development environment**
   ```bash
   # If using devenv.sh
   devenv shell
   
   # Or install dependencies manually
   ```

3. **Build the project**
   ```bash
   ./bin/build
   ```

4. **Run tests**
   ```bash
   ./bin/test
   ```

## Documentation Workflow

### Generating Documentation

StreamWeave uses Doxidize for documentation generation:

1. **Install Doxidize** (if not already installed)
   ```bash
   cargo install doxidize
   ```

2. **Generate documentation**
   ```bash
   ./bin/docs
   ```

   This will:
   - Try to use Doxidize if available
   - Fall back to `cargo doc` if Doxidize is not available
   - Output documentation to `target/doc/` or `target/doc/doxidize/`

3. **View documentation**
   ```bash
   # Open in browser (cargo doc)
   cargo doc --open
   
   # Or navigate to target/doc/doxidize/index.html (Doxidize)
   ```

### Documentation Standards

- All public APIs must have doc comments (`///`)
- Module-level documentation should use `//!`
- Include code examples in doc comments where appropriate
- Follow Rust documentation conventions
- Run `cargo doc` to verify documentation compiles

### Updating Documentation

1. Make your code changes
2. Add or update doc comments
3. Run `./bin/docs` to regenerate documentation
4. Review the generated docs
5. Commit both code and documentation changes

## Code Style

- Follow Rust style guidelines
- Run `cargo fmt` before committing
- Run `cargo clippy` to check for issues
- Use `./bin/lint` to run all linting checks

## Testing

- Write tests for new functionality
- Ensure all tests pass: `./bin/test`
- Aim for >90% test coverage
- Include integration tests for complex features

## Pull Request Process

1. Create a feature branch
2. Make your changes
3. Ensure all tests pass
4. Update documentation if needed
5. Submit a pull request with a clear description

## Documentation Review

When submitting PRs that affect public APIs:
- Ensure all new public items have doc comments
- Verify documentation generates correctly
- Check that examples in docs compile and run
- Update relevant guides if API changes affect usage

