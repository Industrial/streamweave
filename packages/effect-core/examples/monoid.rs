use effect_core::traits::monoid::Monoid;
use effect_core::traits::semigroup::Semigroup; // Monoid extends Semigroup for `combine`

fn main() {
  // Example 1: String Monoid (Concatenation)
  let s1 = "hello ".to_string();
  let s2 = "world".to_string();
  let s_empty = String::empty();

  // combine (from Semigroup)
  let combined_s = s1.clone().combine(s2.clone());
  assert_eq!(combined_s, "hello world");

  // empty (identity)
  assert_eq!(s1.clone().combine(s_empty.clone()), s1);
  assert_eq!(s_empty.clone().combine(s1.clone()), s1);

  // mconcat
  let strings = vec![s1.clone(), s2.clone(), "!".to_string()];
  let mconcat_s = String::mconcat(strings);
  assert_eq!(mconcat_s, "hello world!");

  println!("String Monoid examples executed successfully!");

  // Example 2: Vec<i32> Monoid (Concatenation)
  let v1 = vec![1, 2];
  let v2 = vec![3, 4];
  let v_empty = Vec::<i32>::empty();

  // combine (from Semigroup)
  let combined_v = v1.clone().combine(v2.clone());
  assert_eq!(combined_v, vec![1, 2, 3, 4]);

  // empty (identity)
  assert_eq!(v1.clone().combine(v_empty.clone()), v1);
  assert_eq!(v_empty.clone().combine(v1.clone()), v1);

  // mconcat
  let vecs = vec![v1.clone(), v2.clone(), vec![5, 6]];
  let mconcat_v = Vec::mconcat(vecs);
  assert_eq!(mconcat_v, vec![1, 2, 3, 4, 5, 6]);

  println!("Vec<i32> Monoid (concatenation) examples executed successfully!");

  // Example 3: i32 Monoid (Addition)
  let n1: i32 = 10;
  let n2: i32 = 5;
  let n_empty = i32::empty(); // Should be 0 for i32 under addition

  // combine (from Semigroup) - This is addition for i32
  let combined_n = n1.combine(n2);
  assert_eq!(combined_n, 15);

  // empty (identity for addition)
  assert_eq!(n_empty, 0);
  assert_eq!(n1.combine(n_empty), n1);
  assert_eq!(n_empty.combine(n1), n1);

  // mconcat (summing a list of numbers)
  let numbers = vec![1, 2, 3, 4, 5];
  let sum_n = i32::mconcat(numbers);
  assert_eq!(sum_n, 15);

  println!("i32 Monoid (addition) examples executed successfully!");
}
