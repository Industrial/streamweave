//! Constants and format helpers for the StreamWeave–Mermaid convention.
//!
//! Edge labels use the format `source_port->target_port` with backslash escaping for `\` and `->`.
//! Metadata and graph I/O use comment lines prefixed with [`COMMENT_PREFIX`].

/// Prefix for StreamWeave-specific comment lines in `.mmd` files.
pub const COMMENT_PREFIX: &str = "%% streamweave:";

/// Literal separator between source and target port names in an edge label.
pub const EDGE_LABEL_SEPARATOR: &str = "->";

/// Formats an edge label from source and target port names.
///
/// Escapes backslash as `\\` and `->` as `\->` in each port name, then joins with `->`.
#[must_use]
pub fn format_edge_label(source_port: &str, target_port: &str) -> String {
  format!("{}->{}", escape_port(source_port), escape_port(target_port))
}

/// Escapes a port name for use in an edge label segment.
///
/// Replaces `\` with `\\` and `->` with `\->`.
#[must_use]
pub fn escape_port(port: &str) -> String {
  let mut out = String::with_capacity(port.len() + 4);
  let mut i = 0;
  let bytes = port.as_bytes();
  while i < bytes.len() {
    if bytes[i] == b'\\' {
      out.push('\\');
      out.push('\\');
      i += 1;
    } else if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'>' {
      out.push_str("\\->");
      i += 2;
    } else {
      out.push(char::from(bytes[i]));
      i += 1;
    }
  }
  out
}

/// Parses an edge label into `(source_port, target_port)`.
///
/// Splits on the first unescaped `->`, then unescapes each segment. Returns `None` if the label
/// does not contain an unescaped `->`.
#[must_use]
pub fn parse_edge_label(label: &str) -> Option<(String, String)> {
  let mut i = 0;
  let bytes = label.as_bytes();
  let mut first_end = None;
  while i < bytes.len() {
    if bytes[i] == b'\\' && i + 1 < bytes.len() {
      i += 2;
      continue;
    }
    if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'>' {
      first_end = Some(i);
      break;
    }
    i += 1;
  }
  let sep_at = first_end?;
  let left = unescape_port(&label[..sep_at]);
  let right = unescape_port(&label[sep_at + 2..]);
  Some((left, right))
}

/// Unescapes a port name segment from an edge label.
///
/// Replaces `\\` with `\` and `\->` with `->`.
#[must_use]
pub fn unescape_port(segment: &str) -> String {
  let mut out = String::with_capacity(segment.len());
  let mut i = 0;
  let bytes = segment.as_bytes();
  while i < bytes.len() {
    if bytes[i] == b'\\' && i + 1 < bytes.len() {
      if bytes[i + 1] == b'\\' {
        out.push('\\');
        i += 2;
      } else if bytes[i + 1] == b'-' && i + 2 < bytes.len() && bytes[i + 2] == b'>' {
        out.push_str("->");
        i += 3;
      } else {
        out.push(char::from(bytes[i]));
        i += 1;
      }
    } else {
      out.push(char::from(bytes[i]));
      i += 1;
    }
  }
  out
}

/// Returns whether the line is a StreamWeave comment line (after trim).
#[must_use]
pub fn is_streamweave_comment(line: &str) -> bool {
  line.trim().starts_with(COMMENT_PREFIX)
}

/// Strips the comment prefix and returns the payload (trimmed), or `None` if not a streamweave comment.
#[must_use]
pub fn streamweave_comment_payload(line: &str) -> Option<&str> {
  line.trim().strip_prefix(COMMENT_PREFIX).map(|s| s.trim())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn format_and_parse_simple() {
    let label = format_edge_label("out", "in");
    assert_eq!(label, "out->in");
    assert_eq!(parse_edge_label(&label), Some(("out".into(), "in".into())));
  }

  #[test]
  fn format_and_parse_escape_arrow() {
    let label = format_edge_label("out", "->");
    assert_eq!(label, "out->\\->");
    assert_eq!(parse_edge_label(&label), Some(("out".into(), "->".into())));
  }

  #[test]
  fn format_and_parse_escape_backslash() {
    let label = format_edge_label("a\\b", "c");
    assert_eq!(label, "a\\\\b->c");
    assert_eq!(parse_edge_label(&label), Some(("a\\b".into(), "c".into())));
  }

  #[test]
  fn parse_missing_separator() {
    assert_eq!(parse_edge_label("noarrow"), None);
  }

  #[test]
  fn is_streamweave_comment_accepts_prefix() {
    assert!(is_streamweave_comment(
      "%% streamweave: execution_mode=deterministic"
    ));
    assert!(is_streamweave_comment("  %% streamweave: input x -> n.p"));
    assert!(!is_streamweave_comment("%% other comment"));
  }

  #[test]
  fn streamweave_comment_payload_returns_rest() {
    assert_eq!(
      streamweave_comment_payload("%% streamweave: execution_mode=deterministic"),
      Some("execution_mode=deterministic")
    );
  }
}
