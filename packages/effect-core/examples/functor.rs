use effect_core::traits::functor::Functor;

fn main() {
  // map on Some using Functor trait
  let some_value: Option<i32> = Some(5);
  let mapped_some = <Option<i32> as Functor<i32>>::map(some_value, |x| x * 2);
  assert_eq!(mapped_some, Some(10));

  // map on None using Functor trait
  let none_value: Option<i32> = None;
  let mapped_none = <Option<i32> as Functor<i32>>::map(none_value, |x| x * 2); // The closure won't be called
  assert_eq!(mapped_none, None);

  // map to a different type using Functor trait
  let some_string_value: Option<String> = Some("hello".to_string());
  let mapped_to_length = <Option<String> as Functor<String>>::map(some_string_value, |s| s.len());
  assert_eq!(mapped_to_length, Some(5));

  println!("Functor example for Option works!");
}
