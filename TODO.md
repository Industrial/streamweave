# EffectRust TODO List

## Functional Programming Primitives to Implement

### 0. Missing Trait Implementations
#### Semigroup
- [x] `char` - Using character concatenation
- [x] `Box<T> where T: Semigroup` - Delegating to inner type
- [x] `Arc<T> where T: Semigroup` - Delegating to inner type
- [x] `Rc<T> where T: Semigroup` - Implemented as LocalSemigroup in local module (not thread-safe)
- [x] `Cow<'_, T> where T: Semigroup + ToOwned` - Delegating to inner type

#### Monoid
- [x] `char` - With empty as a space or null character
- [x] `Box<T> where T: Monoid` - Delegating to inner type
- [x] `Arc<T> where T: Monoid` - Delegating to inner type
- [x] `Rc<T> where T: Monoid` - Delegating to inner type
- [x] `Cow<'_, T> where T: Monoid + ToOwned` - Delegating to inner type

#### Category
- [x] `BTreeMap<K, V>` - Using BTreeMapCategory proxy with vector-based grouping
- [x] `char` - For character transformations
- [x] `&str` - For string slice operations
- [x] `String` - For string operations
- [x] `i32`, `i64`, `f32`, `f64` - For numeric transformations
- [ ] `LinkedList<T>` - For element identity and sequence operations
- [ ] `VecDeque<T>` - For element identity and sequence operations
- [ ] `Cow<'_, T> where T: Category` - Delegating to inner type
- [ ] `BTreeSet<T>` - Based on set membership

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
