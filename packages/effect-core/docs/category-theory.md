# Category Theory Guide

This document provides a comprehensive introduction to the category theory concepts used in Effect Core.

## Overview

Category theory is a branch of mathematics that studies abstract structures and relationships between them. In functional programming, category theory provides a mathematical foundation for understanding and designing abstractions.

## Core Concepts

### Categories

A category consists of:
- **Objects**: Types or sets
- **Morphisms**: Functions or transformations between objects
- **Composition**: A way to combine morphisms
- **Identity**: A special morphism for each object

In Rust, categories are represented by the `Category` trait:

```rust
pub trait Category<A: CloneableThreadSafe, B: CloneableThreadSafe> {
    fn id() -> impl Fn(A) -> B;
    fn compose<C, F, G>(f: F, g: G) -> impl Fn(A) -> C
    where
        F: Fn(B) -> C + CloneableThreadSafe,
        G: Fn(A) -> B + CloneableThreadSafe;
}
```

### Functors

A functor is a mapping between categories that preserves structure. In functional programming, functors are types that can be mapped over.

**Functor Laws:**
1. **Identity**: `functor.map(|x| x) == functor`
2. **Composition**: `functor.map(|x| g(f(x))) == functor.map(f).map(g)`

```rust
pub trait Functor<T: CloneableThreadSafe>: Category<T, T> {
    type HigherSelf<U: CloneableThreadSafe>: CloneableThreadSafe + Functor<U>;
    
    fn map<U, F>(self, f: F) -> Self::HigherSelf<U>
    where
        F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
        U: CloneableThreadSafe;
}
```

### Applicatives

Applicatives extend functors with the ability to apply functions in context.

**Applicative Laws:**
1. **Identity**: `pure(id).apply(v) == v`
2. **Composition**: `pure(compose).apply(u).apply(v).apply(w) == u.apply(v.apply(w))`
3. **Homomorphism**: `pure(f).apply(pure(x)) == pure(f(x))`
4. **Interchange**: `u.apply(pure(y)) == pure(|f| f(y)).apply(u)`

```rust
pub trait Applicative<A: CloneableThreadSafe>: Functor<A> {
    fn pure(a: A) -> Self;
    
    fn apply<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
    where
        F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
        B: CloneableThreadSafe;
}
```

### Monads

Monads extend applicatives with the ability to sequence computations with context.

**Monad Laws:**
1. **Left Identity**: `pure(a).bind(f) == f(a)`
2. **Right Identity**: `m.bind(pure) == m`
3. **Associativity**: `m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))`

```rust
pub trait Monad<A: CloneableThreadSafe>: Applicative<A> {
    fn bind<B, F>(self, f: F) -> Self::HigherSelf<B>
    where
        F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
        B: CloneableThreadSafe;
}
```

### Arrows

Arrows are generalizations of functions that support composition and product operations.

```rust
pub trait Arrow<A: CloneableThreadSafe, B: CloneableThreadSafe>: Category<A, B> {
    fn arr<F>(f: F) -> Self
    where
        F: Fn(A) -> B + CloneableThreadSafe;
    
    fn first<C>(self) -> impl Arrow<(A, C), (B, C)>
    where
        C: CloneableThreadSafe;
    
    fn second<C>(self) -> impl Arrow<(C, A), (C, B)>
    where
        C: CloneableThreadSafe;
}
```

### Bifunctors

Bifunctors are functors with two parameters.

```rust
pub trait Bifunctor<A: CloneableThreadSafe, B: CloneableThreadSafe> {
    type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe>: CloneableThreadSafe + Bifunctor<C, D>;
    
    fn bimap<C, D, F, G>(self, f: F, g: G) -> Self::HigherSelf<C, D>
    where
        F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
        G: for<'a> FnMut(&'a B) -> D + CloneableThreadSafe,
        C: CloneableThreadSafe,
        D: CloneableThreadSafe;
}
```

### Comonads

Comonads are the dual of monads, representing computations that can extract context.

```rust
pub trait Comonad<A: CloneableThreadSafe>: Functor<A> {
    fn extract(self) -> A;
    
    fn extend<B, F>(self, f: F) -> Self::HigherSelf<B>
    where
        F: for<'a> FnMut(&'a Self) -> B + CloneableThreadSafe,
        B: CloneableThreadSafe;
}
```

## Mathematical Laws

### Functor Laws

```rust
// Identity law
proptest! {
    #[test]
    fn test_functor_identity_law(xs in prop::collection::vec(any::<i32>(), 0..100)) {
        let functor = xs.clone();
        let result = functor.map(|x| x);
        prop_assert_eq!(result, xs);
    }
}

// Composition law
proptest! {
    #[test]
    fn test_functor_composition_law(
        xs in prop::collection::vec(any::<i32>(), 0..100),
        f_idx in 0..INT_FUNCTIONS.len(),
        g_idx in 0..INT_FUNCTIONS.len()
    ) {
        let functor = xs.clone();
        let f = INT_FUNCTIONS[f_idx];
        let g = INT_FUNCTIONS[g_idx];
        
        // functor.map(f).map(g) == functor.map(|x| g(f(x)))
        let result1 = functor.clone().map(f).map(g);
        let result2 = functor.map(|x| g(&f(x)));
        
        prop_assert_eq!(result1, result2);
    }
}
```

### Monad Laws

```rust
// Left identity law
proptest! {
    #[test]
    fn test_monad_left_identity(
        a in any::<i32>(),
        f_idx in 0..MONAD_FUNCTIONS.len()
    ) {
        let f = MONAD_FUNCTIONS[f_idx];
        let result1 = Some(a).bind(f);
        let result2 = f(&a);
        prop_assert_eq!(result1, result2);
    }
}

// Right identity law
proptest! {
    #[test]
    fn test_monad_right_identity(xs in prop::collection::vec(any::<i32>(), 0..100)) {
        let monad = Some(xs.clone());
        let result = monad.bind(|x| Some(x.clone()));
        prop_assert_eq!(result, Some(xs));
    }
}
```

## Type Classes and Instances

### Standard Library Types

#### Option

```rust
impl<T: CloneableThreadSafe> Functor<T> for Option<T> {
    type HigherSelf<U: CloneableThreadSafe> = Option<U>;
    
    fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
    where
        F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
        U: CloneableThreadSafe,
    {
        match self {
            Some(x) => Some(f(&x)),
            None => None,
        }
    }
}
```

#### Result

```rust
impl<T: CloneableThreadSafe, E: CloneableThreadSafe> Functor<T> for Result<T, E> {
    type HigherSelf<U: CloneableThreadSafe> = Result<U, E>;
    
    fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
    where
        F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
        U: CloneableThreadSafe,
    {
        match self {
            Ok(x) => Ok(f(&x)),
            Err(e) => Err(e),
        }
    }
}
```

#### Vec

```rust
impl<T: CloneableThreadSafe> Functor<T> for Vec<T> {
    type HigherSelf<U: CloneableThreadSafe> = Vec<U>;
    
    fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
    where
        F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
        U: CloneableThreadSafe,
    {
        self.into_iter().map(|x| f(&x)).collect()
    }
}
```

## Advanced Concepts

### Higher-Kinded Types

We simulate higher-kinded types using Generic Associated Types (GATs):

```rust
pub trait Functor<T: CloneableThreadSafe> {
    type HigherSelf<U: CloneableThreadSafe>: CloneableThreadSafe + Functor<U>;
}
```

### Natural Transformations

Natural transformations are mappings between functors:

```rust
pub trait NaturalTransformation<F, G, A>
where
    F: Functor<A>,
    G: Functor<A>,
{
    fn transform(self) -> G::HigherSelf<A>;
}
```

### Adjunctions

Adjunctions are relationships between functors:

```rust
pub trait Adjunction<F, G, A, B>
where
    F: Functor<A>,
    G: Functor<B>,
{
    fn left_adjoint(self) -> F::HigherSelf<A>;
    fn right_adjoint(self) -> G::HigherSelf<B>;
}
```

## Practical Applications

### Error Handling

Monads provide elegant error handling:

```rust
// Using Result as a monad
let result = Some(5)
    .bind(|x| if *x > 0 { Some(100 / x) } else { None })
    .bind(|x| if *x < 50 { Some(x * 2) } else { None });
```

### Async Programming

Futures can be treated as monads:

```rust
// Using Future as a monad
let future = async { 5 }
    .bind(|x| async { x * 2 })
    .bind(|x| async { x + 1 });
```

### State Management

State can be managed using state monads:

```rust
// State monad example
let state = State::new(0)
    .bind(|s| State::new(s + 1))
    .bind(|s| State::new(s * 2));
```

## Conclusion

Category theory provides a solid mathematical foundation for functional programming. The abstractions in Effect Core are designed to:

- **Preserve Structure**: Maintain mathematical relationships
- **Ensure Correctness**: Follow category theory laws
- **Enable Composition**: Allow natural composition of operations
- **Provide Generality**: Work across different types and contexts

Understanding these concepts helps in using the library effectively and designing new abstractions. 