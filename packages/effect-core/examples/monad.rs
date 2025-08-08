use effect_core::traits::applicative::Applicative;
use effect_core::traits::monad::Monad; // Monad extends Applicative, pure is needed.

fn main() {
  // Example with Option<i32>

  // Using bind (or flat_map)
  let option1: Option<i32> = Some(5);
  let result_bind = option1.bind(|x| if *x > 0 { Some(*x * 2) } else { None });
  assert_eq!(result_bind, Some(10));

  let option2: Option<i32> = Some(-5);
  let result_bind_none = option2.bind(|x| if *x > 0 { Some(*x * 2) } else { None });
  assert_eq!(result_bind_none, None);

  let option3: Option<i32> = None;
  let result_bind_none2 = option3.bind(|x: &i32| if *x > 0 { Some(*x * 2) } else { None });
  assert_eq!(result_bind_none2, None);

  // Using then
  let option4: Option<i32> = Some(100);
  let option5: Option<String> = Some("hello".to_string());
  let result_then = option4.then(option5.clone()); // Clone option5 as `then` takes ownership
  assert_eq!(result_then, Some("hello".to_string()));

  let option6: Option<i32> = None;
  let option7: Option<String> = Some("world".to_string());
  let result_then_none = option6.then(option7.clone());
  assert_eq!(result_then_none, None);

  let option8: Option<i32> = Some(20);
  let option9: Option<String> = None;
  let result_then_none2 = option8.then(option9.clone());
  assert_eq!(result_then_none2, None);

  // Monad laws:
  // 1. Left identity: pure(a).bind(f) == f(a)
  let f = |x: &i32| Some(*x + 1);
  let a = 10;
  assert_eq!(Option::<i32>::pure(a).bind(f), f(&a));

  // 2. Right identity: m.bind(pure) == m
  let m: Option<i32> = Some(20);
  assert_eq!(m.clone().bind(|x| Option::<i32>::pure(*x)), m);

  let m_none: Option<i32> = None;
  assert_eq!(m_none.clone().bind(|x| Option::<i32>::pure(*x)), m_none);

  // 3. Associativity: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  let g = |y: &i32| Some(*y * 2);
  let m2: Option<i32> = Some(5);

  let left_assoc = m2.clone().bind(f).bind(g);
  let right_assoc = m2.clone().bind(move |x| f(x).bind(g));
  assert_eq!(left_assoc, right_assoc);
  assert_eq!(left_assoc, Some(12));

  let m2_none: Option<i32> = None;
  let left_assoc_none = m2_none.clone().bind(f).bind(g);
  let right_assoc_none = m2_none.clone().bind(move |x| f(x).bind(g));
  assert_eq!(left_assoc_none, right_assoc_none);
  assert_eq!(left_assoc_none, None);

  println!("Monad examples with Option<i32> executed successfully!");
}
