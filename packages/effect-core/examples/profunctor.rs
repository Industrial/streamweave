use effect_core::traits::profunctor::Profunctor;
use effect_core::types::pair::Pair;
use effect_core::types::threadsafe::CloneableThreadSafe;

// Helper functions - these are Send + Sync + 'static + Clone
fn add_10_fn(x: i32) -> i32 {
  x + 10
}
fn to_string_fn(x: i32) -> String {
  x.to_string()
}
fn append_exclamation_fn(s: String) -> String {
  format!("{}!", s)
}
fn parse_int_fn(s: String) -> i32 {
  s.parse().unwrap_or(0)
}

// Dummy panicking function for the backward part of Pair
fn panic_fn_dummy<T: CloneableThreadSafe, R: CloneableThreadSafe>(_: T) -> R {
  panic!("Dummy backward function called")
}

fn main() {
  // Original profunctor: Pair<i32, String>
  let profunctor = Pair::new(to_string_fn, panic_fn_dummy::<String, i32>);
  assert_eq!(profunctor.apply(123), "123".to_string());

  // dimap: Results in Pair<String, String>
  let dimapped_profunctor = profunctor.dimap(parse_int_fn, append_exclamation_fn);
  assert_eq!(
    dimapped_profunctor.apply("42".to_string()),
    "42!".to_string()
  );

  // lmap: Results in Pair<String, String>
  let profunctor_lmap = Pair::new(to_string_fn, panic_fn_dummy::<String, i32>);
  let lmapped_profunctor = profunctor_lmap.lmap(parse_int_fn);
  assert_eq!(lmapped_profunctor.apply("55".to_string()), "55".to_string());

  // rmap: Results in Pair<i32, String>
  let profunctor_rmap = Pair::new(to_string_fn, panic_fn_dummy::<String, i32>);
  let rmapped_profunctor = profunctor_rmap.rmap(append_exclamation_fn);
  assert_eq!(rmapped_profunctor.apply(77), "77!".to_string());

  // Example with different types for rmap:
  // Original: Pair<i32, i32>
  let profunctor_add = Pair::new(add_10_fn, panic_fn_dummy::<i32, i32>);
  let rmapped_add_to_string = profunctor_add.rmap(to_string_fn);
  assert_eq!(rmapped_add_to_string.apply(5), "15".to_string());

  println!("Profunctor examples with Pair struct executed successfully!");
}
