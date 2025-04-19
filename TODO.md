# EffectRust TODO List

## Functional Programming Primitives to Implement

### 0. Missing Trait Implementations
#### Semigroup
- [ ] `char` - Using character concatenation
- [ ] `Box<T> where T: Semigroup` - Delegating to inner type
- [ ] `Arc<T> where T: Semigroup` - Delegating to inner type
- [ ] `Rc<T> where T: Semigroup` - Delegating to inner type
- [ ] `Cow<'_, T> where T: Semigroup + ToOwned` - Delegating to inner type

#### Monoid
- [ ] `char` - With empty as a space or null character
- [ ] `Box<T> where T: Monoid` - Delegating to inner type
- [ ] `Arc<T> where T: Monoid` - Delegating to inner type
- [ ] `Rc<T> where T: Monoid` - Delegating to inner type
- [ ] `Cow<'_, T> where T: Monoid + ToOwned` - Delegating to inner type

#### Category
- [ ] `BTreeMap<K, V>` - Based on key lookup and value identity
- [ ] `BTreeSet<T>` - Based on set membership
- [ ] `LinkedList<T>` - For element identity and sequence operations
- [ ] `VecDeque<T>` - For element identity and sequence operations
- [ ] `String` - For string operations
- [ ] `char` - For character transformations
- [ ] `&str` - For string slice operations
- [ ] `Cow<'_, T> where T: Category` - Delegating to inner type

#### Functor
- [ ] `Rc<T>` - Similar to the implementation for `Arc<T>`
- [ ] `VecDeque<T>` - For mapping over elements
- [ ] `LinkedList<T>` - For mapping over elements
- [ ] `BTreeSet<T>` - For mapping set elements
- [ ] `Cow<'_, T> where T: Functor` - Delegating to inner type

#### Foldable
- [ ] `VecDeque<T>` - For folding operations
- [ ] `LinkedList<T>` - For folding over linked lists
- [ ] `HashMap<K, V>` - For folding over key-value pairs (implementation exists but is commented out)
- [ ] `String` - For folding over characters
- [ ] `Rc<T>` - Similar to the implementation for `Arc<T>`
- [ ] `Cow<'_, T> where T: Foldable` - Delegating to inner type

#### Applicative
- [ ] `HashMap<K, V>` - For applying functions to map values
- [ ] `BTreeMap<K, V>` - For applying functions to map values
- [ ] `Rc<T>` - Similar to the implementation for `Arc<T>`
- [ ] `VecDeque<T>` - For sequence-based applicative operations
- [ ] `LinkedList<T>` - For sequence-based applicative operations
- [ ] `Cow<'_, T> where T: Applicative` - Delegating to inner type

#### Bifunctor
- [ ] `HashMap<K, V>` - For mapping over both keys and values
- [ ] `BTreeMap<K, V>` - For mapping over both keys and values
- [ ] `Rc<(A, B)>` - Wrapping tuple bifunctor
- [ ] `Arc<(A, B)>` - Wrapping tuple bifunctor
- [ ] `Box<(A, B)>` - Wrapping tuple bifunctor
- [ ] `Cow<'_, (A, B)>` - For borrowed or owned tuples

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

## TODO
- For each TODO item:
  - Implement it.
  - Add 100% covering tests for all permutations.
  - Fix any issues and keep running the tests until all tests are green.
  - Make a git commit (use --no-verify).

## Tests
- Generate 100% covering tests for all permutations.
- Run the tests for this module only.
- Fix any issues and keep running the tests until all tests are green.

## Now
1. Given our current implemented traits, pick a file from _old and move it to the traits.
2. Remove all the impls and tests and leave only the trait.
3. For all rust basic types that this trait should have an impl for, including ones not currently covered for any trait in the `impls`, implement those without tests.

## Tasks.md
1. In the traits directory, create a TASKS.md file or empty it. This file will contain the "state" of the tasks that are left to complete.
2. Inside the TASKS.md file you will put:
    1. [ ] Given our current implemented traits, pick a file from _old and move it to the traits.
    2. [ ] Remove all the impls and tests and leave only the trait.
3. For all rust basic types that this trait should have an impl for, including ones not currently covered for any trait in the `impls`, add a to-do entry with `[ ]` to the TASKS.md file. This file should not include unit tests (yet).
    3.1 For each entry you generated, add a sub-task that will tell the LLM to use `TEST.md` for adding 100% coverage of all permutations of the implementation.
4. When everything has been completed, you should complete all to-do items one by one.