use effect_core::traits::arrow::Arrow;
use effect_core::types::morphism::Morphism;

fn main() {
  // Create two simple morphisms (functions)
  let add_one = Morphism::<i32, i32>::arrow(|x: i32| x + 1);
  let double = Morphism::<i32, i32>::arrow(|x: i32| x * 2);

  // Test arrow (lifting a function into a morphism)
  let another_morphism = Morphism::<i32, String>::arrow(|x: i32| x.to_string());
  assert_eq!(another_morphism.apply(5), "5");

  // Test split
  // split(add_one, double) creates a new morphism that takes a pair (i32, i32)
  // and returns a pair (i32, i32) where the first element is (input.0 + 1)
  // and the second element is (input.1 * 2)
  let split_morphism = <Morphism<i32, i32> as Arrow<i32, i32>>::split::<i32, i32, (), ()>(
    add_one.clone(),
    double.clone(),
  );
  assert_eq!(split_morphism.apply((5, 10)), (6, 20));

  // Test fanout
  // fanout(add_one, double) creates a new morphism that takes an i32
  // and returns a pair (i32, i32) where the first element is (input + 1)
  // and the second element is (input * 2)
  let fanout_morphism = <Morphism<i32, i32> as Arrow<i32, i32>>::fanout::<i32>(add_one, double);
  assert_eq!(fanout_morphism.apply(5), (6, 10));

  println!("Arrow example for Morphism works!");
}
