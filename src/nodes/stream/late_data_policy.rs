//! # Late Data Policy for Event-Time Windows
//!
//! Configures how events arriving after the watermark (late data) are handled.
//!
//! See [docs/windowing.md](../../../docs/windowing.md) §5.6.

/// Policy for handling late data (events with event_time &lt; watermark).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LateDataPolicy {
  /// Discard late events (default).
  #[default]
  Drop,

  /// Route late events to a separate `"late"` output port for separate handling.
  SideOutput,
}

impl LateDataPolicy {
  /// Returns true if late events should be sent to the side output port.
  pub fn use_side_output(&self) -> bool {
    matches!(self, LateDataPolicy::SideOutput)
  }

  /// Returns true if late events should be dropped (no side output).
  pub fn is_drop(&self) -> bool {
    matches!(self, LateDataPolicy::Drop)
  }
}
