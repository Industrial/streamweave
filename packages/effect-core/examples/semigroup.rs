use effect_core::traits::semigroup::Semigroup;

fn main() {
  // Example 1: String Semigroup (Concatenation)
  let s1 = "hello ".to_string();
  let s2 = "world".to_string();
  let s3 = "!".to_string();

  // combine
  let combined_s = s1.clone().combine(s2.clone());
  assert_eq!(combined_s, "hello world");

  // Associativity: (s1 . s2) . s3 == s1 . (s2 . s3)
  let assoc_left_s = (s1.clone().combine(s2.clone())).combine(s3.clone());
  let assoc_right_s = s1.clone().combine(s2.clone().combine(s3.clone()));
  assert_eq!(assoc_left_s, assoc_right_s);
  assert_eq!(assoc_left_s, "hello world!");

  println!("String Semigroup examples executed successfully!");

  // Example 2: Vec<i32> Semigroup (Concatenation)
  let v1 = vec![1, 2];
  let v2 = vec![3, 4];
  let v3 = vec![5, 6];

  // combine
  let combined_v = v1.clone().combine(v2.clone());
  assert_eq!(combined_v, vec![1, 2, 3, 4]);

  // Associativity: (v1 . v2) . v3 == v1 . (v2 . v3)
  let assoc_left_v = (v1.clone().combine(v2.clone())).combine(v3.clone());
  let assoc_right_v = v1.clone().combine(v2.clone().combine(v3.clone()));
  assert_eq!(assoc_left_v, assoc_right_v);
  assert_eq!(assoc_left_v, vec![1, 2, 3, 4, 5, 6]);

  println!("Vec<i32> Semigroup (concatenation) examples executed successfully!");

  // Example 3: i32 Semigroup (Addition with wrapping)
  // The numeric implementation uses wrapping_add.
  let n1: i32 = 10;
  let n2: i32 = 5;
  let n3: i32 = 7;

  // combine
  let combined_n = n1.combine(n2);
  assert_eq!(combined_n, 15);

  // Associativity: (n1 + n2) + n3 == n1 + (n2 + n3)
  let assoc_left_n = (n1.combine(n2)).combine(n3);
  let assoc_right_n = n1.combine(n2.combine(n3));
  assert_eq!(assoc_left_n, assoc_right_n);
  assert_eq!(assoc_left_n, 22);

  // Wrapping behavior example
  let max_i32 = i32::MAX;
  let wrapped_sum = max_i32.combine(1);
  assert_eq!(wrapped_sum, i32::MIN);

  println!("i32 Semigroup (addition with wrapping) examples executed successfully!");
}
