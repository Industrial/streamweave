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
- [x] alternative/monadplus operations (empty, alt, some, many)
- [x] natural transformations (transform)
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

## Tests
Run the tests and fix issues. Keep going until all tests pass.

## Tasks.md
1. In the traits directory, create a TASKS.md file or empty it. This file will contain the "state" of the tasks that are left to complete.
2. Inside the TASKS.md file you will put:
    1. [ ] Given our current implemented traits, pick a file from _old and move it to the traits.
    2. [ ] Remove all the impls and tests and leave only the trait.
3. For all rust basic types that this trait should have an impl for, including ones not currently covered for any trait in the `impls`, add a to-do entry with `[ ]` to the TASKS.md file. This file should not include unit tests (yet).
    3.1 For each entry you generated, add a sub-task that will tell the LLM to use `TEST.md` for adding 100% coverage of all permutations of the implementation.
4. When everything has been completed, you should complete all to-do items one by one.