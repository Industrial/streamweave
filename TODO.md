# EffectRust TODO List

## Functional Programming Primitives to Implement

### 1. Core Functional Primitives
- [x] map/flatMap/filter equivalents
- [x] fold/reduce operations
- [x] zip/zipWith operations
- [x] scan operations
- [x] take/drop operations
- [x] partition operations
- [x] groupBy operations
- [x] distinct operations
- [x] sliding window operations
- [x] merge/concat operations
- [x] interleave operations
- [x] buffer operations
- [x] throttle/debounce operations
- [x] monoid operations (mempty, mappend, mconcat)
- [x] semigroup operations (combine)
- [x] category theory primitives (id, compose, arr, first, second)
- [x] bifunctor operations (bimap, first, second)
- [x] contravariant operations (contramap)
- [x] comonad operations (extract, duplicate, extend)
- [ ] alternative/monadplus operations (empty, alt, some, many)
- [ ] natural transformations (transform)
- [ ] free monad operations (pure, foldMap, hoist)
- [ ] profunctor operations (dimap, lmap, rmap)
- [ ] foldable1/traversable1 operations (fold1, sequence1, toNonEmpty)
- [ ] monad transformers (lift, run)

### 2. Layer System
- [ ] Layer concept for dependency injection and context management
- [ ] Layer.provide and Layer.merge equivalents
- [ ] Layer trait with composition capabilities

### 3. Schedule & Policy
- [ ] Retry scheduling with backoff policies
- [ ] Schedule.exponential and Schedule.fibonacci equivalents
- [ ] Schedule trait with various retry strategies

### 4. Resource Management
- [ ] Scoped resources
- [ ] Resource pooling
- [ ] Automatic cleanup with finalizers
- [ ] Resource composition

### 5. Fiber System
- [ ] Fiber-based concurrency primitives
- [ ] Fiber.fork equivalent
- [ ] Fiber.join equivalent
- [ ] Fiber.interrupt equivalent
- [ ] Fiber supervision

### 6. Queue Abstractions
- [ ] Bounded/Unbounded queues
- [ ] Priority queues
- [ ] Sliding/Dropping queues

### 7. Ref Types
- [ ] Ref.make equivalent
- [ ] Ref.modify equivalent
- [ ] Atomic references
- [ ] Software transactional memory

### 8. Error Handling
- [ ] Typed error channels
- [ ] Error defects
- [ ] Error recovery policies
- [ ] Error tagging and refinement

### 9. Runtime Configuration
- [ ] Config composition
- [ ] Environment variables integration
- [ ] Secret management
- [ ] Runtime flags

### 10. Metrics & Telemetry
- [ ] Metrics collection
- [ ] Tracing
- [ ] Logging integration
- [ ] Health checks

### 11. Type-level Programming
- [ ] More type-level computations
- [ ] Better type inference
- [ ] Type-level constraints
- [ ] Type-level error handling

### 12. Stream Combinators
- [ ] Chunking
- [ ] Windowing
- [ ] Rate limiting
- [ ] Backpressure handling

### 13. Testing Utilities
- [ ] Test environments
- [ ] Clock manipulation
- [ ] Random number generation
- [ ] Property testing integration

### 14. Caching
- [ ] Memoization
- [ ] Cache policies
- [ ] Cache invalidation
- [ ] Distributed caching

### 15. Circuit Breaking
- [ ] Failure detection
- [ ] Recovery strategies
- [ ] Circuit state management
- [ ] Circuit metrics

### 16. Semaphore & Lock Primitives
- [ ] Fair semaphores
- [ ] Read-write locks
- [ ] Distributed locks
- [ ] Lock-free algorithms 

- [ ] Check out https://github.com/viperproject/prusti-dev
- [ ] Check out https://github.com/creusot-rs/creusot?tab=readme-ov-file

1. Ask for each of the files in `effect-core`:
    1.1 Which traits/implementations should be compositions of others in the `packages/*` that haven't been done yet.
        1.1.1 Make sure each trait gets their own file.
        1.1.2 Impl of Trait is implemented in the same file.
    1.2 Which basic Rust type system implementations of the trait haven't been done yet.
    1.3 Use PropTest for the tests.
        1.4 Make sure all possible permutations are tested.
        1.5 100% Test Coverage.
2. Run `bin/test` after every change you make. Don't run tests with `cargo test`. Fix all errors and report when all tests are green.
3. If everything above leaves no files to be updated, read `TODO.md` and look at the next item and implement that.

For each item above give me the rundown and then apply them.
