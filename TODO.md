# EffectRust TODO List (Transposed)

## Types to Implement

### Box<T>
- [x] Semigroup (where T: Semigroup) - Delegating to inner type
- [x] Monoid (where T: Monoid) - Delegating to inner type
- [x] Bifunctor for Box<(A, B)> - Wrapping tuple bifunctor

### Arc<T>
- [x] Semigroup (where T: Semigroup) - Delegating to inner type
- [x] Monoid (where T: Monoid) - Delegating to inner type
- [ ] Bifunctor for Arc<(A, B)> - Wrapping tuple bifunctor

### Rc<T>
- [x] Semigroup - Implemented as LocalSemigroup in local module (not thread-safe)
- [x] Monoid - Delegating to inner type
- [ ] Functor - Similar to the implementation for Arc<T>
- [ ] Foldable - Similar to the implementation for Arc<T>
- [ ] Applicative - Similar to the implementation for Arc<T>
- [ ] Bifunctor for Rc<(A, B)> - Wrapping tuple bifunctor

### Cow<'_, T>
- [x] Semigroup (where T: Semigroup + ToOwned) - Delegating to inner type
- [x] Monoid (where T: Monoid + ToOwned) - Delegating to inner type
- [ ] Category (where T: Category) - Delegating to inner type
- [ ] Functor (where T: Functor) - Delegating to inner type
- [ ] Foldable (where T: Foldable) - Delegating to inner type
- [ ] Applicative (where T: Applicative) - Delegating to inner type
- [ ] Bifunctor for Cow<'_, (A, B)> - For borrowed or owned tuples

### char
- [x] Semigroup - Using character concatenation
- [x] Monoid - With empty as a space or null character
- [x] Category - For character transformations

### Numeric Types
- [x] Category for i32, i64, f32, f64 - For numeric transformations

### String Types
- [x] Category for &str - For string slice operations
- [x] Category for String - For string operations
- [ ] Foldable for String - For folding over characters

### BTreeMap<K, V>
- [x] Category - Using BTreeMapCategory proxy with vector-based grouping
- [ ] Applicative - For applying functions to map values
- [ ] Bifunctor - For mapping over both keys and values

### HashMap<K, V>
- [ ] Foldable - For folding over key-value pairs (implementation exists but is commented out)
- [ ] Applicative - For applying functions to map values
- [ ] Bifunctor - For mapping over both keys and values

### Collections
- [ ] Category for LinkedList<T> - For element identity and sequence operations
- [ ] Category for VecDeque<T> - For element identity and sequence operations
- [ ] Category for BTreeSet<T> - Based on set membership
- [ ] Functor for VecDeque<T> - For mapping over elements
- [ ] Functor for LinkedList<T> - For mapping over elements
- [ ] Functor for BTreeSet<T> - For mapping set elements
- [ ] Foldable for VecDeque<T> - For folding operations
- [ ] Foldable for LinkedList<T> - For folding over linked lists
- [ ] Applicative for VecDeque<T> - For sequence-based applicative operations
- [ ] Applicative for LinkedList<T> - For sequence-based applicative operations
