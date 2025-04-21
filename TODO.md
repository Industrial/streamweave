# EffectRust TODO List (Transposed)

## Types to Implement

### Box<T>
- [x] Semigroup (where T: Semigroup) - Delegating to inner type
- [x] Monoid (where T: Monoid) - Delegating to inner type
- [x] Bifunctor for Box<(A, B)> - Wrapping tuple bifunctor

### Arc<T>
- [x] Semigroup (where T: Semigroup) - Delegating to inner type
- [x] Monoid (where T: Monoid) - Delegating to inner type
- [x] Bifunctor for Arc<(A, B)> - Wrapping tuple bifunctor

### char
- [x] Semigroup - Using character concatenation
- [x] Monoid - With empty as a space or null character
- [x] Category - For character transformations

### Numeric Types
- [x] Category for i32, i64, f32, f64 - For numeric transformations

### String Types
- [x] Category for &str - For string slice operations
- [x] Category for String - For string operations
- [x] Foldable for String - For folding over characters

### Cow<'_, T>
- [x] Semigroup (where T: Semigroup + ToOwned) - Delegating to inner type
- [x] Monoid (where T: Monoid + ToOwned) - Delegating to inner type
- [x] Category (where T: Category) - Delegating to inner type
- [ ] Functor (where T: Functor) - Delegating to inner type (implementation uses CowMapper and CowVecFunctor wrapper types)
- [ ] Foldable (where T: Foldable) - Delegating to inner type
- [ ] Applicative (where T: Applicative) - Delegating to inner type
- [ ] Bifunctor for Cow<'_, (A, B)> - For borrowed or owned tuples

### BTreeMap<K, V>
- [x] Category - Using BTreeMapCategory proxy with vector-based grouping
- [ ] Applicative - For applying functions to map values
- [ ] Bifunctor - For mapping over both keys and values

### HashMap<K, V>
- [ ] Foldable - For folding over key-value pairs (implementation exists but is commented out)
- [ ] Applicative - For applying functions to map values
- [ ] Bifunctor - For mapping over both keys and values

### Collections
- [x] Category for LinkedList<T> - For element identity and sequence operations
- [x] Category for VecDeque<T> - For element identity and sequence operations
- [x] Category for BTreeSet<T> - Based on set membership
- [x] Functor for VecDeque<T> - For mapping over elements
- [x] Functor for LinkedList<T> - For mapping over elements
- [x] Functor for BTreeSet<T> - For mapping set elements
- [ ] Foldable for VecDeque<T> - For folding operations
- [ ] Foldable for LinkedList<T> - For folding over linked lists
- [ ] Applicative for VecDeque<T> - For sequence-based applicative operations
- [ ] Applicative for LinkedList<T> - For sequence-based applicative operations
