use crate::traits::monoid::Monoid;
use std::path::PathBuf;

impl Monoid for PathBuf {
  fn empty() -> Self {
    PathBuf::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    assert_eq!(PathBuf::empty(), PathBuf::new());
    assert!(PathBuf::empty().as_os_str().is_empty());
  }

  #[test]
  fn test_empty_right_identity() {
    let path = PathBuf::from("/tmp");
    let empty = PathBuf::empty();

    // Empty path on the right should not change the original path
    assert_eq!(path.clone().combine(empty), path);
  }

  #[test]
  fn test_empty_left_identity_with_relative() {
    // For relative paths, empty path on the left is an identity
    let empty = PathBuf::empty();
    let relative = PathBuf::from("tmp");

    assert_eq!(empty.combine(relative.clone()), relative);
  }

  #[test]
  fn test_empty_left_identity_with_absolute() {
    // For absolute paths, empty path on the left is also an identity
    let empty = PathBuf::empty();
    let absolute = PathBuf::from("/tmp");

    assert_eq!(empty.combine(absolute.clone()), absolute);
  }

  #[test]
  fn test_mconcat_empty_iterator() {
    let paths: Vec<PathBuf> = vec![];
    assert_eq!(PathBuf::mconcat(paths), PathBuf::empty());
  }

  #[test]
  fn test_mconcat_with_relative_paths() {
    // Joining a sequence of relative paths
    let paths = vec![
      PathBuf::from("first"),
      PathBuf::from("second"),
      PathBuf::from("third"),
    ];

    assert_eq!(PathBuf::mconcat(paths), PathBuf::from("first/second/third"));
  }

  #[test]
  fn test_mconcat_with_mixed_paths() {
    // When an absolute path is encountered, it resets the path up to that point
    let paths = vec![
      PathBuf::from("first"),
      PathBuf::from("/absolute"),
      PathBuf::from("after"),
    ];

    assert_eq!(PathBuf::mconcat(paths), PathBuf::from("/absolute/after"));
  }

  proptest! {
    #[test]
    fn prop_empty_is_right_identity(path in "[a-zA-Z0-9_/]{1,20}") {
      let path_buf = PathBuf::from(path);
      prop_assert_eq!(path_buf.clone().combine(PathBuf::empty()), path_buf);
    }

    #[test]
    fn prop_empty_is_left_identity_for_relative(
      // Generate relative paths (no leading slash)
      rel_path in "[a-zA-Z0-9_]{1,10}(/[a-zA-Z0-9_]{1,10}){0,3}"
    ) {
      let path_buf = PathBuf::from(rel_path);
      prop_assert_eq!(PathBuf::empty().combine(path_buf.clone()), path_buf);
    }

    #[test]
    fn prop_mconcat_equivalent_to_fold(
      segments in proptest::collection::vec("[a-zA-Z0-9_]{1,5}", 0..5)
    ) {
      let paths: Vec<PathBuf> = segments.into_iter().map(PathBuf::from).collect();

      let mconcat_result = PathBuf::mconcat(paths.clone());
      let fold_result = paths.into_iter().fold(PathBuf::empty(), |acc, x| acc.combine(x));

      prop_assert_eq!(mconcat_result, fold_result);
    }
  }
}
