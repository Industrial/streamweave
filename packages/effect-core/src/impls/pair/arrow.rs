use crate::traits::arrow::Arrow;
use crate::types::pair::Pair;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Arrow<A, B> for Pair<A, B> {
    fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
    where
        F: Fn(C) -> D + CloneableThreadSafe,
    {
        Pair::new(
            f,
            |_: D| panic!("Cannot create inverse of arbitrary function"),
        )
    }

    fn split<
        C: CloneableThreadSafe,
        D: CloneableThreadSafe,
        E: CloneableThreadSafe,
        G: CloneableThreadSafe,
    >(
        f: Self::Morphism<A, B>,
        g: Self::Morphism<C, D>,
    ) -> Self::Morphism<(A, C), (B, D)> {
        let f_forward = f.clone();
        let g_forward = g.clone();
        Pair::new(
            move |(a, c)| (f_forward.apply(a), g_forward.apply(c)),
            move |(b, d)| (f.unapply(b), g.unapply(d)),
        )
    }

    fn fanout<C: CloneableThreadSafe>(
        f: Self::Morphism<A, B>,
        g: Self::Morphism<A, C>,
    ) -> Self::Morphism<A, (B, C)> {
        let f_forward = f.clone();
        let g_forward = g.clone();
        Pair::new(
            move |a: A| {
                let a_clone = a.clone();
                (f_forward.apply(a), g_forward.apply(a_clone))
            },
            move |(b, _): (B, C)| {
                // For the unapply of fanout, we need to agree on which inverse to use.
                // Here we choose to use the inverse from f, but this is arbitrary.
                f.unapply(b)
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::category::Category;
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
    fn test_arrow_creation() {
        let f = |x: i32| x + 1;
        let arrow_f = <Pair<i32, i32> as Arrow<i32, i32>>::arrow(f);
        
        let input = 5;
        let output = arrow_f.apply(input);
        
        assert_eq!(output, 6);
    }

    #[test]
    fn test_split() {
        let f = Pair::new(|x: i32| x + 1, |x: i32| x - 1);
        let g = Pair::new(|x: i32| x * 2, |x: i32| x / 2);
        
        let split = <Pair<i32, i32> as Arrow<i32, i32>>::split::<i32, i32, (), ()>(f, g);
        
        let input = (5, 10);
        let output = split.apply(input);
        
        assert_eq!(output, (6, 20)); // (5+1, 10*2)
        assert_eq!(split.unapply(output), input);
    }

    #[test]
    fn test_fanout() {
        let f = Pair::new(|x: i32| x + 1, |x: i32| x - 1);
        let g = Pair::new(|x: i32| x * 2, |x: i32| x / 2);
        
        let fanout = <Pair<i32, i32> as Arrow<i32, i32>>::fanout(f.clone(), g.clone());
        
        let input = 5;
        let output = fanout.apply(input);
        
        assert_eq!(output, (6, 10)); // (5+1, 5*2)
        
        // For fanout, the unapply is somewhat arbitrary since we have two inverses
        // We test that it reverses at least one direction
        let result = fanout.unapply(output);
        assert_eq!(result, f.unapply(output.0));
    }

    proptest! {
        #[test]
        fn test_arrow_split_law_prop(
            x in -1000..1000i32,
            y in -1000..1000i32,
            f_idx in 0..FUNCTIONS.len(),
            g_idx in 0..FUNCTIONS.len()
        ) {
            let f = FUNCTIONS[f_idx];
            let g = FUNCTIONS[g_idx];
            
            // Create pairs with forward and reverse functions
            let f1 = f;
            let g1 = g;
            let f_inverse = move |y| if f1(y-1) == y { y-1 } else { y };
            let g_inverse = move |y| if g1(y-1) == y { y-1 } else { y };
            
            let pair_f = Pair::new(f, f_inverse);
            let pair_g = Pair::new(g, g_inverse);

            // Split the arrows
            let split_arrows = <Pair<i32, i32> as Arrow<i32, i32>>::split::<i32, i32, (), ()>(pair_f.clone(), pair_g.clone());

            // Create a direct function for splitting
            let direct_split = move |(a, c)| (f(a), g(c));
            let direct = <Pair<(i32, i32), (i32, i32)> as Arrow<(i32, i32), (i32, i32)>>::arrow(direct_split);

            // Test with specific input
            let pair = (x, y);
            let result1 = split_arrows.apply(pair);
            let result2 = direct.apply(pair);

            // Both approaches should yield the same result for apply
            prop_assert_eq!(result1, result2);
        }
        
        #[test]
        fn test_arrow_fanout_law_prop(
            x in -1000..1000i32,
            f_idx in 0..FUNCTIONS.len(),
            g_idx in 0..FUNCTIONS.len()
        ) {
            let f = FUNCTIONS[f_idx];
            let g = FUNCTIONS[g_idx];
            
            // Create pairs with forward and reverse functions
            let f1 = f;
            let g1 = g;
            let f_inverse = move |y| if f1(y-1) == y { y-1 } else { y };
            let g_inverse = move |y| if g1(y-1) == y { y-1 } else { y };
            
            let pair_f = Pair::new(f, f_inverse);
            let pair_g = Pair::new(g, g_inverse);

            // Create fanout using arrow implementation
            let fanout_arrows = <Pair<i32, i32> as Arrow<i32, i32>>::fanout(pair_f.clone(), pair_g.clone());

            // Alternative implementation using split and duplicate
            let duplicate = <Pair<i32, i32> as Arrow<i32, i32>>::arrow(|x: i32| (x, x));
            let split_arrows = <Pair<i32, i32> as Arrow<i32, i32>>::split::<i32, i32, (), ()>(pair_f.clone(), pair_g.clone());
            let compose_result = <Pair<i32, i32> as Category<i32, i32>>::compose(duplicate, split_arrows);
            
            // Test with specific input
            let result1 = fanout_arrows.apply(x);
            let result2 = compose_result.apply(x);
            
            // Both approaches should yield the same result for apply
            prop_assert_eq!(result1, result2);
        }
    }

    #[test]
    fn test_arrow_laws() {
        // Test the arrow law: arrow(id) = id
        let identity = |x: i32| x;
        let arrow_id = <Pair<i32, i32> as Arrow<i32, i32>>::arrow(identity);
        let category_id = <Pair<i32, i32> as Category<i32, i32>>::id();
        
        let x = 42;
        assert_eq!(arrow_id.apply(x), category_id.apply(x));
        
        // Test law: arrow(f >>> g) = arrow(f) >>> arrow(g)
        let f = |x: i32| x + 1;
        let g = |x: i32| x * 2;
        let fg = move |x: i32| g(f(x));
        
        let arrow_f = <Pair<i32, i32> as Arrow<i32, i32>>::arrow(f);
        let arrow_g = <Pair<i32, i32> as Arrow<i32, i32>>::arrow(g);
        let arrow_fg = <Pair<i32, i32> as Arrow<i32, i32>>::arrow(fg);
        
        let composed = <Pair<i32, i32> as Category<i32, i32>>::compose(arrow_f, arrow_g);
        
        let x = 5;
        assert_eq!(arrow_fg.apply(x), composed.apply(x));
    }
} 