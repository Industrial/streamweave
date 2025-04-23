# Migration Plan: _old to traits/impls

## Overview
This document outlines the plan to migrate code from the `_old` directory to the modern `traits` and `impls` structure.

## Current Structure
- Files in `_old` contain both trait definitions and implementations
- Modern structure separates traits (`packages/effect-core/src/traits`) and implementations (`packages/effect-core/src/impls`)

## Migration Steps

### 1. Traits Migration
For each file in `_old`:
- [ ] Extract trait definition
- [ ] Update to use `CloneableThreadSafe` instead of `Send + Sync + 'static`
- [ ] Create corresponding file in `traits/` directory
- [ ] Update trait definition to match modern style
- [ ] Add to `traits/mod.rs` exports

### 2. Implementations Migration
For each implementation in the original files:
- [ ] Identify the type being implemented
- [ ] Create/update directory in `impls/` for that type if it doesn't exist
- [ ] Create corresponding implementation file (e.g., `functor.rs`) in that directory
- [ ] Update implementation to use the new trait
- [ ] Add to type's `mod.rs` exports

## Traits to Migrate
- [x] alternative.rs
  - [x] Trait definition (traits/alternative.rs)
  - [x] Option implementation (impls/option/alternative.rs)
  - [x] Result implementation (impls/result/alternative.rs)
- [x] arrow.rs 
  - [x] Trait definition (traits/arrow.rs)
  - [x] Morphism implementation (impls/morphism/arrow.rs)
- [ ] bufferable.rs
- [ ] comonad.rs
- [ ] contravariant.rs
- [ ] distinctable.rs
- [ ] effect.rs
- [ ] either.rs
- [ ] filterable.rs
- [ ] function.rs
- [ ] groupable.rs
- [ ] interleaveable.rs
- [ ] monad.rs
- [ ] monad_plus.rs
- [ ] natural.rs
- [ ] pair.rs
- [ ] partitionable.rs
- [ ] profunctor.rs
- [ ] scannable.rs
- [ ] takeable.rs
- [ ] throttleable.rs
- [ ] traversable.rs
- [ ] windowable.rs
- [ ] zippable.rs

## Implementation Notes
- Follow existing patterns for trait definitions
- Ensure thread safety with CloneableThreadSafe
- Maintain property-based tests
- Update imports to match new structure
