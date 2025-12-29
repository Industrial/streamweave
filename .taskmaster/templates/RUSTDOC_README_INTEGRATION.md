# Rustdoc README Integration Pattern

This document describes the standard pattern for including package README files in rustdoc-generated documentation.

## Pattern

To include a package's README.md file in the rustdoc documentation, add the following line at the very top of the package's `lib.rs` file (before any other code or doc comments):

```rust
#![doc = include_str!("../README.md")]
```

## Example

Here's a complete example of how a `lib.rs` file should be structured:

```rust
#![doc = include_str!("../README.md")]

// Additional module-level documentation can go here if needed
//! Additional implementation details...

pub mod my_module;

pub use my_module::*;
```

## Placement

The `#![doc = include_str!("../README.md")]` line must be:
- At the very top of the file (before any other code)
- Before any `//!` module-level doc comments
- Before any `pub mod` declarations
- Before any other attributes

## File Structure

The README.md file should be located in the package root directory, one level up from `src/lib.rs`:

```
packages/
└── my-package/
    ├── README.md          # Package README
    ├── Cargo.toml
    └── src/
        └── lib.rs         # Contains: #![doc = include_str!("../README.md")]
```

## Benefits

1. **Single Source of Truth**: The README serves as both the package documentation and the rustdoc introduction
2. **Consistency**: All packages follow the same documentation pattern
3. **Discoverability**: Users browsing docs.rs will see the full README content
4. **Maintainability**: Only one file needs to be updated when documentation changes

## When to Apply

This pattern should be applied:
- When creating a new package README
- When updating an existing package to include README in rustdoc
- As part of the package documentation setup process

## Verification

To verify the pattern is working correctly:

1. Generate documentation:
   ```bash
   cargo doc --package my-package
   ```

2. Open the documentation:
   ```bash
   cargo doc --open --package my-package
   ```

3. Check that the README content appears at the top of the package documentation page

## Notes

- The README.md file must exist before this pattern can be applied
- The path `../README.md` assumes the standard package structure
- If the README contains relative links, they should be written to work both in the repository and in the generated docs
- Code blocks in the README will be rendered as code examples in rustdoc

## Integration with Template

When using the `package_readme_template.md` template to create a README:
1. Create the README.md file in the package root
2. Add `#![doc = include_str!("../README.md")]` to the top of `src/lib.rs`
3. Verify the documentation generates correctly

This ensures that the README content is automatically included in the rustdoc output.

