use effect_core::traits::category::Category;

fn main() {
  // Type alias for String category operations for brevity
  type StringCategory = String;

  // arr: Lift functions into morphisms
  let to_upper = <StringCategory as Category<String, String>>::arr(|s: &String| s.to_uppercase());
  let append_exclamation =
    <StringCategory as Category<String, String>>::arr(|s: &String| format!("{}!", s));

  // compose: Compose two morphisms
  let to_upper_then_exclaim = <StringCategory as Category<String, String>>::compose(
    to_upper.clone(),
    append_exclamation.clone(),
  );
  assert_eq!(to_upper_then_exclaim.apply("hello".to_string()), "HELLO!");

  // id: Identity morphism
  let id_morphism = <StringCategory as Category<String, String>>::id::<String>();
  assert_eq!(id_morphism.apply("world".to_string()), "world");

  // Test composition with id (left identity: id . f = f)
  let id_then_to_upper =
    <StringCategory as Category<String, String>>::compose(id_morphism.clone(), to_upper.clone());
  assert_eq!(id_then_to_upper.apply("test".to_string()), "TEST");

  // Test composition with id (right identity: f . id = f)
  let to_upper_then_id =
    <StringCategory as Category<String, String>>::compose(to_upper.clone(), id_morphism.clone());
  assert_eq!(to_upper_then_id.apply("another".to_string()), "ANOTHER");

  // first: applies a morphism to the first element of a pair
  let first_morphism =
    <StringCategory as Category<String, String>>::first::<String, String, i32>(to_upper.clone());
  let pair_input: (String, i32) = ("value".to_string(), 42);
  assert_eq!(first_morphism.apply(pair_input), ("VALUE".to_string(), 42));

  // second: applies a morphism to the second element of a pair
  let second_morphism = <StringCategory as Category<String, String>>::second::<String, String, i32>(
    append_exclamation.clone(),
  );
  let pair_input_2: (i32, String) = (123, "string".to_string());
  assert_eq!(
    second_morphism.apply(pair_input_2),
    (123, "string!".to_string())
  );

  println!("Category example for String works!");
}
