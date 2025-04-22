# EffectRust TODO List (Transposed)

## Types to Implement

### Box<T>
- [x] Bifunctor for Box<(A, B)> - Wrapping tuple bifunctor
- [x] Monoid (where T: Monoid) - Delegating to inner type
- [x] Semigroup (where T: Semigroup) - Delegating to inner type

### Arc<T>
- [x] Bifunctor for Arc<(A, B)> - Wrapping tuple bifunctor
- [x] Monoid (where T: Monoid) - Delegating to inner type
- [x] Semigroup (where T: Semigroup) - Delegating to inner type

### char
- [x] Category - For character transformations
- [x] Monoid - With empty as a space or null character
- [x] Semigroup - Using character concatenation

### Numeric Types
- [x] Category for i32, i64, f32, f64 - For numeric transformations

### String Types
- [x] Category for &str - For string slice operations
- [x] Category for String - For string operations
- [x] Foldable for String - For folding over characters

### Cow<'_, T>
- [x] Applicative (where T: Applicative) - Delegating to inner type
- [x] Bifunctor for Cow<'_, (A, B)> - For borrowed or owned tuples
- [ ] Foldable (where T: Foldable) - Delegating to inner type
- [x] Category (where T: Category) - Delegating to inner type
- [x] Functor (where T: Functor) - Delegating to inner type (implementation uses CowMapper and CowVecFunctor wrapper types)
- [x] Monoid (where T: Monoid + ToOwned) - Delegating to inner type
- [x] Semigroup (where T: Semigroup + ToOwned) - Delegating to inner type

### BTreeMap<K, V>
- [x] Bifunctor - For mapping over both keys and values
- [x] Applicative - For applying functions to map values
- [x] Category - Using BTreeMapCategory proxy with vector-based grouping
- [x] Functor - For mapping over elements

### HashMap<K, V>
- [x] Bifunctor - For mapping over both keys and values
- [ ] Foldable - For folding over key-value pairs (implementation exists but is commented out)
- [x] Applicative - For applying functions to map values

### Collections
- [ ] Foldable for LinkedList<T> - For folding over linked lists
- [ ] Foldable for VecDeque<T> - For folding operations
- [x] Applicative for LinkedList<T> - For sequence-based applicative operations
- [x] Applicative for VecDeque<T> - For sequence-based applicative operations
- [x] Category for BTreeSet<T> - Based on set membership
- [x] Category for LinkedList<T> - For element identity and sequence operations
- [x] Category for VecDeque<T> - For element identity and sequence operations
- [x] Functor for BTreeSet<T> - For mapping set elements
- [x] Functor for LinkedList<T> - For mapping over elements
- [x] Functor for VecDeque<T> - For mapping over elements
