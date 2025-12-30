# README Template Usage Guide

This guide explains how to use the `package_readme_template.md` template to create consistent README files for all StreamWeave packages.

## Template Overview

The template is designed to be adaptable for different package types:
- **Core packages**: Foundation packages (streamweave, error, message, offset, transaction)
- **Producer packages**: Data source implementations (array, vec, file, stdio, etc.)
- **Consumer packages**: Data sink implementations (file, stdio, database, etc.)
- **Transformer packages**: Data transformation implementations (map, filter, reduce, etc.)
- **Integration packages**: External system integrations (kafka, redis, database-*, etc.)

## Template Sections

### Required Sections (All Packages)

1. **Header** - Package name, badges, description
2. **Key Features** - List of main features
3. **Installation** - Cargo.toml dependency
4. **Quick Start** - Minimal working example
5. **API Overview** - Main types/traits
6. **Usage Examples** - 2-3 practical examples
7. **Documentation Links** - Links to docs.rs and repository

### Optional Sections (Package-Specific)

- **Configuration** - For packages with configuration options
- **Architecture** - For complex packages
- **Error Handling** - For packages with specific error handling
- **Performance Considerations** - For performance-critical packages
- **Use Cases** - For packages with specific use cases
- **Dependencies** - Detailed dependency information if needed

## Package Type Guidelines

### Core Packages

**Focus on:**
- Fundamental concepts and abstractions
- Trait definitions and implementations
- Integration with other packages
- Design patterns

**Example sections:**
- API Overview: Focus on traits (Producer, Transformer, Consumer)
- Usage Examples: Show how traits are implemented
- Architecture: Explain the core abstractions

### Producer Packages

**Focus on:**
- Data source setup
- Configuration options
- Reading patterns
- Error handling for I/O operations

**Example sections:**
- Quick Start: Show creating a producer and reading data
- Configuration: Producer-specific configuration
- Usage Examples: Different data sources and patterns

### Consumer Packages

**Focus on:**
- Data sink setup
- Writing patterns
- Output formatting
- Error handling for I/O operations

**Example sections:**
- Quick Start: Show creating a consumer and writing data
- Configuration: Consumer-specific configuration
- Usage Examples: Different output destinations and patterns

### Transformer Packages

**Focus on:**
- Transformation logic
- Configuration options
- Performance characteristics
- Common use cases

**Example sections:**
- Quick Start: Show using the transformer in a pipeline
- API Overview: Transformer-specific methods
- Usage Examples: Different transformation patterns

### Integration Packages

**Focus on:**
- External system setup
- Connection configuration
- Integration patterns
- Error handling for external systems

**Example sections:**
- Prerequisites: External system requirements
- Quick Start: Setup and basic usage
- Configuration: Connection and system-specific options
- Troubleshooting: Common issues and solutions

## Template Variable Guide

Replace these placeholders in the template:

- `{package-name}` - Package name (e.g., `streamweave-csv`)
- `{package-description}` - One-line description
- `{package-tagline}` - Short tagline
- `{package-detailed-description}` - Detailed description (2-3 paragraphs)
- `{version}` - Current version (e.g., `0.6.0`)
- `{key-features-list}` - Bulleted list of features
- `{quick-start-example}` - Minimal working code example
- `{api-overview-description}` - Description of main API
- `{api-examples}` - Code examples for main API
- `{example-1-title}`, `{example-1-description}`, `{example-1-code}` - First example
- `{example-2-title}`, `{example-2-description}`, `{example-2-code}` - Second example
- `{optional-features-section}` - Features section if package has optional features
- `{configuration-section}` - Configuration documentation if applicable
- `{architecture-section}` - Architecture explanation if applicable
- `{dependencies-list}` - List of main dependencies
- `{error-handling-section}` - Error handling patterns if applicable
- `{performance-section}` - Performance notes if applicable
- `{use-cases-section}` - Common use cases if applicable
- `{related-packages-section}` - Links to related packages

## Code Example Guidelines

### Quick Start Example

Should be:
- Minimal (10-20 lines)
- Self-contained
- Copy-paste runnable
- Demonstrates core functionality

### Usage Examples

Should include:
- 2-3 examples showing different patterns
- Real-world use cases
- Error handling where appropriate
- Comments explaining key concepts

## Badge Configuration

All packages should include:
- Crates.io badge: `[![Crates.io](https://img.shields.io/crates/v/{package-name}.svg)](https://crates.io/crates/{package-name})`
- Documentation badge: `[![Documentation](https://docs.rs/{package-name}/badge.svg)](https://docs.rs/{package-name})`
- License badge: `[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)`

## Documentation Links

All READMEs should include:
- Link to docs.rs: `https://docs.rs/{package-name}`
- Link to repository: `https://github.com/Industrial/streamweave/tree/main/packages/{package-name}`
- Link to main documentation: `https://docs.rs/streamweave`

## Consistency Checklist

Before finalizing a README:

- [ ] All template variables replaced
- [ ] Badges are correct and working
- [ ] Code examples compile and run
- [ ] Links to docs.rs work
- [ ] Repository link points to correct subdirectory
- [ ] Consistent formatting with other package READMEs
- [ ] All sections relevant to package type are included
- [ ] No placeholder text remains
- [ ] Examples demonstrate real-world usage
- [ ] Error handling is documented where applicable

## Integration with Rustdoc

After creating the README, add this to the package's `lib.rs`:

```rust
#![doc = include_str!("../README.md")]
```

This will include the README content in the generated rustdoc documentation.

For detailed information about this pattern, see [RUSTDOC_README_INTEGRATION.md](RUSTDOC_README_INTEGRATION.md).

## Example: Producer Package

For a producer package like `streamweave-file`:

1. **Quick Start**: Show reading from a file
2. **API Overview**: Focus on `FileProducer` type
3. **Configuration**: File path, encoding, buffering options
4. **Usage Examples**: Reading different file types, error handling
5. **Error Handling**: File not found, permission errors, etc.

## Example: Transformer Package

For a transformer package like `streamweave-transformers`:

1. **Quick Start**: Show using transformer in a pipeline
2. **API Overview**: Focus on transformer trait and implementations
3. **Configuration**: Transformer-specific options
4. **Usage Examples**: Different transformation patterns
5. **Performance**: Notes on transformation overhead

## Example: Integration Package

For an integration package like `streamweave-kafka`:

1. **Prerequisites**: Kafka setup requirements
2. **Quick Start**: Connection and basic usage
3. **Configuration**: Connection strings, topics, consumer groups
4. **Usage Examples**: Producing, consuming, error handling
5. **Troubleshooting**: Common connection and configuration issues

## Next Steps

After creating a README using this template:

1. Review against this guide
2. Test all code examples
3. Verify all links work
4. Add rustdoc integration (`#![doc = include_str!("../README.md")]` at the top of `lib.rs`)
   - See [RUSTDOC_README_INTEGRATION.md](RUSTDOC_README_INTEGRATION.md) for details
5. Update main README to link to package README

