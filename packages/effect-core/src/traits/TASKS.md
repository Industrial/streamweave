# Implementation Tasks

1. [ ] Given our current implemented traits, pick a file from _old and move it to the traits.
2. [ ] Remove all the impls and tests and leave only the trait.
3. [ ] For all rust basic types that this trait should have an impl for, including ones not currently covered for any trait in the `impls`, implement those without tests.

## Monoid Implementation Tasks

- [x] Implement Monoid for `char` - With empty as a space or null character
  - [x] Add 100% coverage tests for all permutations using TEST.md guidelines
- [x] Implement Monoid for `Box<T> where T: Monoid` - Delegating to inner type
  - [x] Add 100% coverage tests for all permutations using TEST.md guidelines
- [x] Implement Monoid for `Arc<T> where T: Monoid` - Delegating to inner type
  - [x] Add 100% coverage tests for all permutations using TEST.md guidelines
- [x] Implement Monoid for `Rc<T> where T: Monoid` - Delegating to inner type
  - [x] Add 100% coverage tests for all permutations using TEST.md guidelines 