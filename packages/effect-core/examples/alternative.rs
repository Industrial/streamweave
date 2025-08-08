use effect_core::traits::alternative::Alternative;

fn main() {
  let x: Option<i32> = Some(5);
  let y: Option<i32> = None;
  let z: Option<i32> = Some(10);

  // alt chooses the first Some value
  assert_eq!(x.alt(y), Some(5));
  assert_eq!(y.alt(x), Some(5));
  assert_eq!(x.alt(z), Some(5));

  // empty returns None
  let empty_option: Option<i32> = Alternative::empty();
  assert_eq!(empty_option, None);

  assert_eq!(empty_option.alt(x), Some(5));
  assert_eq!(x.alt(empty_option), Some(5));

  println!("Alternative example for Option works!");
}
