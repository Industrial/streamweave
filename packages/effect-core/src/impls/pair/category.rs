use crate::traits::category::Category;
use crate::types::pair::Pair;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Category<A, B> for Pair<A, B> {
    type Morphism<C: CloneableThreadSafe, D: CloneableThreadSafe> = Pair<C, D>;

    fn id<C: CloneableThreadSafe>() -> Self::Morphism<C, C> {
        Pair::new(|x| x, |x| x)
    }

    fn compose<C: CloneableThreadSafe, D: CloneableThreadSafe, E: CloneableThreadSafe>(
        f: Self::Morphism<C, D>,
        g: Self::Morphism<D, E>,
    ) -> Self::Morphism<C, E> {
        let f_forward = f.clone();
        let g_forward = g.clone();
        Pair::new(
            move |x| g_forward.apply(f_forward.apply(x)),
            move |x| f.unapply(g.unapply(x)),
        )
    }

    fn arr<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
    where
        F: for<'a> Fn(&'a C) -> D + CloneableThreadSafe,
    {
        Pair::new(
            move |x| f(&x),
            |_| panic!("Cannot create inverse of arbitrary function"),
        )
    }

    fn first<C: CloneableThreadSafe, D: CloneableThreadSafe, E: CloneableThreadSafe>(
        f: Self::Morphism<C, D>,
    ) -> Self::Morphism<(C, E), (D, E)> {
        let f_forward = f.clone();
        Pair::new(
            move |(x, y)| (f_forward.apply(x), y),
            move |(x, y)| (f.unapply(x), y),
        )
    }

    fn second<C: CloneableThreadSafe, D: CloneableThreadSafe, E: CloneableThreadSafe>(
        f: Self::Morphism<C, D>,
    ) -> Self::Morphism<(E, C), (E, D)> {
        let f_forward = f.clone();
        Pair::new(
            move |(x, y)| (x, f_forward.apply(y)),
            move |(x, y)| (x, f.unapply(y)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Define test functions that are safe and avoid overflow
    const FUNCTIONS: &[fn(i32) -> i32] = &[
        |x| x.saturating_add(1),
        |x| x.saturating_mul(2),
        |x| x.saturating_sub(1),
        |x| if x != 0 { x / 2 } else { 0 },
        |x| x.saturating_mul(x),
        |x| x.checked_neg().unwrap_or(i32::MAX),
    ];

    #[test]
    fn test_category_id() {
        let x = 42;
        let id = <Pair<i32, i32> as Category<i32, i32>>::id();
        assert_eq!(id.apply(x), x);
        assert_eq!(id.unapply(x), x);
    }

    proptest! {
        #[test]
        fn test_category_compose_prop(
            x in -1000..1000i32,
            f_idx in 0..FUNCTIONS.len(),
            g_idx in 0..FUNCTIONS.len()
        ) {
            let f = FUNCTIONS[f_idx];
            let g = FUNCTIONS[g_idx];
            
            let pair_f = Pair::new(f, |y| if y % 2 == 0 { y / 2 } else { y });
            let pair_g = Pair::new(g, |y| if y % 2 == 0 { y / 2 } else { y });
            
            let composed = <Pair<i32, i32> as Category<i32, i32>>::compose(pair_f.clone(), pair_g.clone());
            
            // Test forward composition
            prop_assert_eq!(composed.apply(x), pair_g.apply(pair_f.apply(x)));
            
            // Test backward composition (reversed order)
            let result = composed.unapply(g(f(x)));
            let expected = pair_f.unapply(pair_g.unapply(g(f(x))));
            prop_assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_category_first() {
        let pair = Pair::new(|x: i32| x * 2, |x: i32| x / 2);
        let first = <Pair<i32, i32> as Category<i32, i32>>::first::<i32, i32, String>(pair);
        
        let input = (5, "hello".to_string());
        let output = first.apply(input.clone());
        
        assert_eq!(output, (10, "hello".to_string()));
        assert_eq!(first.unapply(output), input);
    }

    #[test]
    fn test_category_second() {
        let pair = Pair::new(|x: i32| x * 2, |x: i32| x / 2);
        let second = <Pair<i32, i32> as Category<i32, i32>>::second::<i32, i32, String>(pair);
        
        let input = ("hello".to_string(), 5);
        let output = second.apply(input.clone());
        
        assert_eq!(output, ("hello".to_string(), 10));
        assert_eq!(second.unapply(output), input);
    }

    #[test]
    fn test_arr() {
        let f = |x: &i32| *x + 1;
        let pair = <Pair<i32, i32> as Category<i32, i32>>::arr(f);
        
        assert_eq!(pair.apply(5), 6);
        
        // The unapply should panic, but we don't test that directly
    }
} 