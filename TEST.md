### Property Testing Guidelines

When implementing property-based tests with proptest, follow these guidelines:

1. **Function Generation**:
   - Instead of generating functions dynamically, use a fixed set of functions that implement `Debug`
   - Define functions as constants at the module level
   - Use `i64` type instead of `i32` to support a wider range of values
   - Use checked arithmetic operations to prevent overflow
   - Example:
     ```rust
     const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
         |x| x.saturating_add(1),
         |x| x.saturating_mul(2),
         |x| x.saturating_sub(1),
         |x| if *x != 0 { x / 2 } else { 0 },
         |x| x.saturating_mul(*x),
         |x| x.checked_neg().unwrap_or(i64::MAX),
     ];
     ```

2. **Test Strategy**:
   - For function composition tests, generate indices into the function array
   - Limit input value ranges to prevent overflow
   - Example:
     ```rust
     proptest! {
         #[test]
         fn test_composition(
             x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
             f_idx in 0..INT_FUNCTIONS.len(),
             g_idx in 0..INT_FUNCTIONS.len()
         ) {
             let f = INT_FUNCTIONS[f_idx];
             let g = INT_FUNCTIONS[g_idx];
             // Test logic here
         }
     }
     ```

3. **Collection Sizing**:
   - Keep collection sizes small (0..5) to prevent excessive test complexity
   - For collections, ensure all inputs are within safe bounds:
     ```rust
     vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5)
     ```

4. **Implementation-Specific Tests**:
   - Create tests that are specific to each container type's implementation
   - For `Option`: Test handling of `None` values
   - For `Result`: Test error propagation
   - For `Vec`: Test element-wise operations
   - For `Future`: Test async operations
   - For `HashMap`: Test key-value relationship preservation
   - For references types (`Box`, `Rc`, `Arc`): Test shared ownership behavior

5. **Test Coverage**:
   - Test all category laws: identity, composition, and bifunctor laws (first/second)
   - Test with various input values within safe bounds
   - Include implementation-specific edge cases

6. **Example Template**:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use proptest::prelude::*;

       // Define test functions with overflow protection
       const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
           |x| x.saturating_add(1),
           |x| x.saturating_mul(2),
           |x| x.saturating_sub(1),
           |x| if *x != 0 { x / 2 } else { 0 },
           |x| x.saturating_mul(*x),
           |x| x.checked_neg().unwrap_or(i64::MAX),
       ];

       proptest! {
           #[test]
           fn test_identity_law(
               // Use bounded input to prevent overflow
               x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
               f_idx in 0..INT_FUNCTIONS.len()
           ) {
               let f = INT_FUNCTIONS[f_idx];
               // Test identity law (specific to this implementation)
           }

           #[test]
           fn test_composition_law(
               // Use bounded input to prevent overflow
               x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
               f_idx in 0..INT_FUNCTIONS.len(),
               g_idx in 0..INT_FUNCTIONS.len(),
               h_idx in 0..INT_FUNCTIONS.len()
           ) {
               // Test composition law (specific to this implementation)
           }
           
           #[test]
           fn test_first_combinator(
               // Implementation-specific test for the first combinator
           ) {
               // Test based on the specific data structure being tested
           }
           
           #[test]
           fn test_second_combinator(
               // Implementation-specific test for the second combinator
           ) {
               // Test based on the specific data structure being tested
           }
       }
   }
   ```

This approach ensures:
- Type safety and overflow protection
- Specific testing for each container type implementation
- Reasonable test run times
- Maintainable test code
- Clear test intent
- Comprehensive law verification