### Property Testing Guidelines

When implementing property-based tests with proptest, follow these guidelines:

1. **Function Generation**:
   - Instead of generating functions dynamically, use a fixed set of functions that implement `Debug`
   - Define functions as constants at the module level
   - Example:
     ```rust
     const FUNCTIONS: &[fn(i32) -> i32] = &[
         |x| x + 1,
         |x| x * 2,
         |x| x - 1,
         |x| x / 2,
         |x| x * x,
         |x| -x,
     ];
     ```

2. **Test Strategy**:
   - For function composition tests, generate indices into the function array
   - Example:
     ```rust
     proptest! {
         #[test]
         fn test_composition(
             x in any::<i32>(),
             f_idx in 0..FUNCTIONS.len(),
             g_idx in 0..FUNCTIONS.len()
         ) {
             let f = FUNCTIONS[f_idx];
             let g = FUNCTIONS[g_idx];
             // Test logic here
         }
     }
     ```

3. **Test Coverage**:
   - Test all possible combinations of functions in the array
   - Test with various input values
   - Test edge cases explicitly if needed

4. **Best Practices**:
   - Keep the function set small but diverse
   - Choose functions that are easy to reason about
   - Avoid functions that might cause overflow or other edge cases
   - Document why each function was chosen

5. **Example Template**:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use proptest::prelude::*;

       // Define test functions
       const FUNCTIONS: &[fn(i32) -> i32] = &[
           |x| x + 1,
           |x| x * 2,
           |x| x - 1,
       ];

       proptest! {
           #[test]
           fn test_identity(x in any::<i32>()) {
               let id = |x: i32| x;
               // Test identity law
           }

           #[test]
           fn test_composition(
               x in any::<i32>(),
               f_idx in 0..FUNCTIONS.len(),
               g_idx in 0..FUNCTIONS.len()
           ) {
               let f = FUNCTIONS[f_idx];
               let g = FUNCTIONS[g_idx];
               // Test composition law
           }
       }
   }
   ```

This approach ensures:
- Type safety
- Debug trait implementation
- Good test coverage
- Maintainable test code
- Clear test intent