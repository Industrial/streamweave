use effect_core::traits::comonad::Comonad;

fn main() {
  let boxed_value: Box<i32> = Box::new(42);

  // extract: Get the value out of the Box
  let extracted_value = boxed_value.clone().extract(); // clone because extract consumes self
  assert_eq!(extracted_value, 42);

  // extend: Apply a function that takes the Comonad (Box) and returns a new value
  // The result is a new Comonad (Box) containing that new value.
  let extended_boxed_value = boxed_value.extend(|b: Box<i32>| *b * 2);
  assert_eq!(*extended_boxed_value, 84);

  // duplicate: Duplicates the comonadic structure
  let initial_box = Box::new(10);
  let duplicated_box: Box<Box<i32>> = initial_box.duplicate();
  assert_eq!(*duplicated_box.extract(), 10);

  println!("Comonad example for Box works!");
}
