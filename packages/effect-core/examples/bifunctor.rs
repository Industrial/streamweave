use effect_core::traits::bifunctor::Bifunctor;

fn main() {
  let tuple: (i32, String) = (10, "hello".to_string());

  // bimap applies functions to both elements of the tuple
  let bimapped_tuple = tuple.clone().bimap(|x| x + 5, |s| s.to_uppercase());
  assert_eq!(bimapped_tuple, (15, "HELLO".to_string()));

  // first applies a function to the first element only
  let first_mapped_tuple = tuple.clone().first(|x| x * 2);
  assert_eq!(first_mapped_tuple, (20, "hello".to_string()));

  // second applies a function to the second element only
  let second_mapped_tuple = tuple.second(|s| s.len());
  assert_eq!(second_mapped_tuple, (10, 5));

  println!("Bifunctor example for tuple works!");
}
