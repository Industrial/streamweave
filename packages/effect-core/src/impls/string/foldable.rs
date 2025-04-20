use crate::impls::string::category::CharCategory;
use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;

impl Foldable<char> for CharCategory {
  fn fold<A, F>(self, init: A, _f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a char) -> A + CloneableThreadSafe,
  {
    // This is a dummy implementation since CharCategory is just a marker type
    // We don't actually have characters in it to fold over
    init
  }

  fn fold_right<A, F>(self, init: A, _f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a char, A) -> A + CloneableThreadSafe,
  {
    // This is a dummy implementation since CharCategory is just a marker type
    init
  }

  fn reduce<F>(self, _f: F) -> Option<char>
  where
    F: for<'a, 'b> FnMut(&'a char, &'b char) -> char + CloneableThreadSafe,
  {
    // This is a dummy implementation since CharCategory is just a marker type
    None
  }

  fn reduce_right<F>(self, _f: F) -> Option<char>
  where
    F: for<'a, 'b> FnMut(&'a char, &'b char) -> char + CloneableThreadSafe,
  {
    // This is a dummy implementation since CharCategory is just a marker type
    None
  }
}

// Extension trait for String providing Foldable functionality
pub trait StringFoldableExt {
  fn fold_chars<A, F>(&self, init: A, f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a char) -> A + CloneableThreadSafe;

  fn fold_right_chars<A, F>(&self, init: A, f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a char, A) -> A + CloneableThreadSafe;

  fn reduce_chars<F>(&self, f: F) -> Option<char>
  where
    F: for<'a, 'b> FnMut(&'a char, &'b char) -> char + CloneableThreadSafe;

  fn reduce_right_chars<F>(&self, f: F) -> Option<char>
  where
    F: for<'a, 'b> FnMut(&'a char, &'b char) -> char + CloneableThreadSafe;
}

impl StringFoldableExt for String {
  fn fold_chars<A, F>(&self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a char) -> A + CloneableThreadSafe,
  {
    let mut acc = init;
    for c in self.chars() {
      acc = f(acc, &c);
    }
    acc
  }

  fn fold_right_chars<A, F>(&self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a char, A) -> A + CloneableThreadSafe,
  {
    let chars: Vec<char> = self.chars().collect();
    let mut acc = init;
    for c in chars.iter().rev() {
      acc = f(c, acc);
    }
    acc
  }

  fn reduce_chars<F>(&self, mut f: F) -> Option<char>
  where
    F: for<'a, 'b> FnMut(&'a char, &'b char) -> char + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    let chars: Vec<char> = self.chars().collect();
    let mut iter = chars.iter();
    let first = *iter.next().unwrap();
    let mut acc = first;

    for c in iter {
      acc = f(&acc, c);
    }

    Some(acc)
  }

  fn reduce_right_chars<F>(&self, mut f: F) -> Option<char>
  where
    F: for<'a, 'b> FnMut(&'a char, &'b char) -> char + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    let chars: Vec<char> = self.chars().collect();
    let mut iter = chars.iter().rev();
    let first = *iter.next().unwrap();
    let mut acc = first;

    for c in iter {
      acc = f(c, &acc);
    }

    Some(acc)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_fold_empty_string() {
    let s = "".to_string();
    let result = s.fold_chars(0, |acc, _| acc + 1);
    assert_eq!(
      result, 0,
      "Folding over an empty string should return the initial value"
    );
  }

  #[test]
  fn test_fold_simple() {
    // Count characters
    let s = "hello".to_string();
    let count = s.clone().fold_chars(0, |acc, _| acc + 1);
    assert_eq!(count, 5, "Should count 5 characters");

    // Sum character codes
    let sum = s.fold_chars(0, |acc, c| acc + (*c as u32));
    assert_eq!(sum, 532, "Sum of 'hello' char codes should be 532");
  }

  #[test]
  fn test_fold_right_simple() {
    // Concatenate characters in reverse (but each character is still a char, not reversed)
    let s = "hello".to_string();
    let result = s.fold_right_chars(String::new(), |c, mut acc| {
      acc.push(*c);
      acc
    });
    assert_eq!(
      result, "olleh",
      "fold_right should process characters from right to left"
    );
  }

  #[test]
  fn test_reduce_simple() {
    // Find the "larger" character (by ASCII/Unicode value)
    let s = "hello".to_string();
    let result = s.reduce_chars(|a, b| if a > b { *a } else { *b });
    assert_eq!(result, Some('o'), "The 'largest' character should be 'o'");

    // Empty string
    let s = "".to_string();
    let result = s.reduce_chars(|a, b| if a > b { *a } else { *b });
    assert_eq!(result, None, "Reducing an empty string should return None");
  }

  #[test]
  fn test_reduce_right_simple() {
    // Find the first character
    let s = "hello".to_string();
    let result = s.reduce_right_chars(|a, _| *a);
    assert_eq!(
      result,
      Some('h'),
      "reduce_right with 'first' should return the first character"
    );

    // Empty string
    let s = "".to_string();
    let result = s.reduce_right_chars(|a, _| *a);
    assert_eq!(result, None, "Reducing an empty string should return None");
  }

  #[test]
  fn test_fold_with_complex_accumulator() {
    // Count occurrences of each character
    let s = "mississippi".to_string();
    let counts = s.fold_chars(std::collections::HashMap::new(), |mut acc, c| {
      *acc.entry(*c).or_insert(0) += 1;
      acc
    });

    assert_eq!(counts.get(&'m'), Some(&1));
    assert_eq!(counts.get(&'i'), Some(&4));
    assert_eq!(counts.get(&'s'), Some(&4));
    assert_eq!(counts.get(&'p'), Some(&2));
  }

  proptest! {
    #[test]
    fn prop_fold_count_matches_length(s in "\\PC*") {
      let s = s.to_string();
      let expected_len = s.chars().count();
      let count = s.fold_chars(0, |acc, _| acc + 1);

      prop_assert_eq!(count, expected_len, "fold should count the same number of characters as chars().count()");
    }

    #[test]
    fn prop_fold_right_count_matches_length(s in "\\PC*") {
      let s = s.to_string();
      let expected_len = s.chars().count();
      let count = s.fold_right_chars(0, |_, acc| acc + 1);

      prop_assert_eq!(count, expected_len, "fold_right should count the same number of characters as chars().count()");
    }

    #[test]
    fn prop_reduce_finds_max_char(s in "[a-z]{1,100}") {
      let s = s.to_string();
      let max_char = s.chars().max();
      let result = s.reduce_chars(|a, b| if a > b { *a } else { *b });

      prop_assert_eq!(result, max_char, "reduce should find the maximum character");
    }

    #[test]
    fn prop_fold_and_fold_right_string_concat(s in "[a-z]{1,20}") {
      let s = s.to_string();

      // Forward concatenation
      let forward = s.clone().fold_chars(String::new(), |mut acc, c| {
        acc.push(*c);
        acc
      });

      // Backward concatenation
      let backward = s.fold_right_chars(String::new(), |c, mut acc| {
        acc.insert(0, *c);
        acc
      });

      prop_assert_eq!(forward, backward, "fold and fold_right with appropriate concatenation should produce the same string");
    }
  }
}
