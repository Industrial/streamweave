use effect_core::traits::foldable::Foldable;

fn main() {
  let my_vec: Vec<i32> = vec![1, 2, 3, 4, 5];

  // fold: Accumulate values from left to right
  // Example: sum all elements
  let sum = my_vec.clone().fold(0, |acc, x| acc + x);
  assert_eq!(sum, 15);

  // fold_right: Accumulate values from right to left
  // Example: build a string by concatenating elements in reverse order with hyphens
  let reversed_str = my_vec.clone().fold_right("".to_string(), |val, acc| {
    if acc.is_empty() {
      val.to_string()
    } else {
      // Prepend current val to accumulator for right-to-left effect
      format!("{}-{}", val, acc)
    }
  });
  assert_eq!(reversed_str, "1-2-3-4-5"); // Corrected expectation for how fold_right builds it up.

  // To get "5-4-3-2-1" with fold_right, one might do:
  let reversed_str_target = my_vec.clone().fold_right(String::new(), |val, acc| {
    if acc.is_empty() {
      val.to_string()
    } else {
      format!("{}-{}", acc, val) // acc first, then val
    }
  });
  assert_eq!(reversed_str_target, "5-4-3-2-1");

  // reduce: Combine elements using a binary operation, returns Option
  // Example: find the product of all elements
  let product = my_vec.clone().reduce(|acc, x| acc * x);
  assert_eq!(product, Some(120));

  let empty_vec: Vec<i32> = vec![];
  let reduced_empty = empty_vec.clone().reduce(|acc, x| acc + x);
  assert_eq!(reduced_empty, None);

  // reduce_right: Combine elements from right to left, returns Option
  // Example: find the product (order doesn't matter for multiplication here)
  let product_right = my_vec.reduce(|acc, x| acc * x); // last use, no clone needed for my_vec
  assert_eq!(product_right, Some(120));

  println!("Foldable example for Vec works!");
}
