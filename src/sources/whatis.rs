//! Fetch a short one-line description from `whatis`.

use std::process::Command;

/// Return the description part of `whatis <cmd>`, if available.
pub fn fetch(cmd: &str) -> Option<String> {
    let output = Command::new("whatis").arg(cmd).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    parse(&raw)
}

fn parse(text: &str) -> Option<String> {
    let first = text.lines().find(|line| !line.trim().is_empty())?.trim();
    let (_, desc) = first.split_once(" - ")?;
    let desc = desc.trim();
    if desc.is_empty() {
        None
    } else {
        Some(desc.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_whatis_description() {
        let text = "cp (1)               - copy files and directories\n";
        assert_eq!(parse(text), Some("copy files and directories".to_string()));
    }
}
