use effect_core::traits::filterable::Filterable;

fn main() {
  let my_vec: Vec<i32> = vec![1, 2, 3, 4, 5, 6];

  // filter_map: Map to Option and keep Some values
  // Example: keep only even numbers and convert them to strings
  let filter_mapped_vec = my_vec.clone().filter_map(|x| {
    if x % 2 == 0 {
      Some(x.to_string())
    } else {
      None
    }
  });
  assert_eq!(
    filter_mapped_vec,
    vec!["2".to_string(), "4".to_string(), "6".to_string()]
  );

  // filter: Keep elements that satisfy a predicate
  // Example: keep only odd numbers
  let filtered_vec = my_vec.filter(|x| x % 2 != 0);
  assert_eq!(filtered_vec, vec![1, 3, 5]);

  println!("Filterable example for Vec works!");
}
