use effect_core::traits::functor::Functor;

fn main() {
  // map on Some
  let some_value: Option<i32> = Some(5);
  let mapped_some = some_value.map(|x| x * 2);
  assert_eq!(mapped_some, Some(10));

  // map on None
  let none_value: Option<i32> = None;
  let mapped_none = none_value.map(|x| x * 2); // The closure won't be called
  assert_eq!(mapped_none, None);

  // map to a different type
  let some_string_value: Option<String> = Some("hello".to_string());
  let mapped_to_length = some_string_value.map(|s| s.len());
  assert_eq!(mapped_to_length, Some(5));

  println!("Functor example for Option works!");
}
