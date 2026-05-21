//! tldr page support (stub — network fetching is out of scope for MVP).
//!
//! This module is reserved for a future implementation that reads locally
//! cached tldr pages (e.g. from `~/.tldr/` or the `tealdeer` cache).

/// Attempt to read a locally-cached tldr page for `cmd`.
///
/// Always returns `None` in the MVP build.
#[allow(dead_code)]
pub fn fetch(_cmd: &str) -> Option<String> {
    None
}
