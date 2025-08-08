use effect_core::traits::applicative::Applicative;

fn main() {
  let x: Option<i32> = Some(5);
  let f: Option<fn(&i32) -> i32> = Some(|x| x + 1);

  // ap applies the function in Some to the value in Some
  assert_eq!(x.ap(f), Some(6));

  let y: Option<i32> = None;
  let g: Option<fn(&i32) -> i32> = Some(|x| x + 1);

  // If either the value or the function is None, ap returns None
  assert_eq!(y.ap(g), None);

  let z: Option<i32> = Some(5);
  let h: Option<fn(&i32) -> i32> = None;
  assert_eq!(z.ap(h), None);

  // pure lifts a value into the Applicative context (Some)
  let pure_value: Option<i32> = Option::<i32>::pure(10);
  assert_eq!(pure_value, Some(10));

  println!("Applicative example for Option works!");
}
