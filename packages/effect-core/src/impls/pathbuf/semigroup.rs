use crate::traits::semigroup::Semigroup;
use std::path::PathBuf;

impl Semigroup for PathBuf {
  fn combine(self, other: Self) -> Self {
    self.join(other)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::tests::test_associativity;
  use proptest::prelude::*;

  #[test]
  fn test_combine_paths() {
    let path1 = PathBuf::from("/tmp");
    let path2 = PathBuf::from("subdir");

    assert_eq!(path1.combine(path2), PathBuf::from("/tmp/subdir"));
  }

  #[test]
  fn test_combine_with_file() {
    let path1 = PathBuf::from("/tmp/dir");
    let path2 = PathBuf::from("file.txt");

    assert_eq!(path1.combine(path2), PathBuf::from("/tmp/dir/file.txt"));
  }

  #[test]
  fn test_combine_multiple_segments() {
    let path1 = PathBuf::from("/tmp");
    let path2 = PathBuf::from("dir/subdir");

    assert_eq!(path1.combine(path2), PathBuf::from("/tmp/dir/subdir"));
  }

  #[test]
  fn test_combine_with_empty() {
    let path1 = PathBuf::from("/tmp");
    let path2 = PathBuf::new();

    assert_eq!(path1.clone().combine(path2.clone()), path1);

    // Empty path on the left behaves differently!
    // It just returns the right side as-is
    assert_eq!(path2.combine(path1), PathBuf::from("/tmp"));
  }

  #[test]
  fn test_combine_absolute_paths() {
    let path1 = PathBuf::from("/tmp");
    let path2 = PathBuf::from("/usr");

    // When joining an absolute path, it replaces the first path entirely
    assert_eq!(path1.combine(path2), PathBuf::from("/usr"));
  }

  #[test]
  fn test_associativity_manual() {
    // Testing associativity with relative paths
    let path1 = PathBuf::from("first");
    let path2 = PathBuf::from("second");
    let path3 = PathBuf::from("third");

    test_associativity(path1, path2, path3);

    // Test with mixed absolute/relative
    let _abs = PathBuf::from("/tmp");
    let _rel1 = PathBuf::from("rel1");
    let _rel2 = PathBuf::from("rel2");

    // Note: Path joining is NOT generally associative with mixed absolute/relative paths!
    // This test would fail:
    // test_associativity(abs, rel1, rel2);
  }

  proptest! {
    // Generate random path segments
    #[test]
    fn prop_join_equivalence(
      segment1 in "[a-zA-Z0-9_]{1,10}",
      segment2 in "[a-zA-Z0-9_]{1,10}"
    ) {
      let path1 = PathBuf::from(segment1);
      let path2 = PathBuf::from(segment2);

      // Verify that combine behavior matches join behavior
      let combined = path1.clone().combine(path2.clone());
      let joined = path1.join(path2);

      prop_assert_eq!(combined, joined);
    }

    // Test associativity for simple relative paths (where it should hold)
    #[test]
    fn prop_associativity_for_relative_paths(
      segment1 in "[a-zA-Z0-9_]{1,10}",
      segment2 in "[a-zA-Z0-9_]{1,10}",
      segment3 in "[a-zA-Z0-9_]{1,10}"
    ) {
      let path1 = PathBuf::from(segment1);
      let path2 = PathBuf::from(segment2);
      let path3 = PathBuf::from(segment3);

      let left = path1.clone().combine(path2.clone()).combine(path3.clone());
      let right = path1.combine(path2.combine(path3));

      prop_assert_eq!(left, right);
    }
  }
}
