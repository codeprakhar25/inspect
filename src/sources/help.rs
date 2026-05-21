//! Fetch and parse `<cmd> --help` output.

use regex::Regex;
use std::process::{Command, Stdio};
use std::time::Duration;

use super::{CommandDocs, Flag};

/// Timeout for `--help` subprocesses.
const HELP_TIMEOUT: Duration = Duration::from_secs(3);

/// Run `<cmd> --help` (capturing stdout + stderr) and parse into [`CommandDocs`].
///
/// Returns `None` when the command is not found at all.
#[allow(dead_code)]
pub fn fetch(cmd: &str) -> Option<CommandDocs> {
    fetch_from(cmd, cmd, &format!("{cmd} --help"))
}

/// Run `exec --help` and label the source separately for display.
pub fn fetch_from(exec: &str, doc_cmd: &str, source_label: &str) -> Option<CommandDocs> {
    let raw = run_help(exec)?;
    Some(parse_text(doc_cmd, &raw, source_label))
}

/// Parse already-captured help text and label the source.
pub fn parse_text(doc_cmd: &str, text: &str, source_label: &str) -> CommandDocs {
    let mut docs = parse(doc_cmd, text);
    docs.sources.push(source_label.to_string());
    docs
}

/// Execute `<cmd> --help` with a timeout, merging stdout and stderr.
fn run_help(cmd: &str) -> Option<String> {
    use std::io::Read;

    // Spawn the process.
    let mut child = Command::new(cmd)
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    // Poll with a crude timeout: we cannot use `std::thread::sleep` in a loop
    // elegantly here without threads, so we use `wait_timeout` via a manual
    // approach using `try_wait`.
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() >= HELP_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
    }

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout_buf);
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr_buf);
    }

    // Prefer stdout; fall back to stderr.
    let combined = if !stdout_buf.is_empty() {
        stdout_buf
    } else {
        stderr_buf
    };

    if combined.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&combined).into_owned())
    }
}

/// Parse `--help` text into [`CommandDocs`].
fn parse(cmd: &str, text: &str) -> CommandDocs {
    let mut docs = CommandDocs::default();

    // Flag line: 1–8 leading spaces then a dash.
    let flag_re = Regex::new(r"^\s{1,8}(-\w[\w-]*(?:,\s*--[\w-]+)?|--[\w-]+)(.*)$")
        .expect("static regex is valid");

    let mut flags: Vec<Flag> = Vec::new();
    let mut saw_usage = false;

    for line in text.lines() {
        let lower = line.to_lowercase();

        // Detect usage line.
        if !saw_usage && (lower.starts_with("usage:") || lower.contains(&format!("usage: {cmd}"))) {
            let usage_part = line
                .find(':')
                .map(|i| line[i + 1..].trim())
                .unwrap_or(line.trim());
            if !usage_part.is_empty() {
                docs.usage = Some(normalize_usage(cmd, usage_part));
            }
            saw_usage = true;
            continue;
        }

        if let Some(caps) = flag_re.captures(line) {
            let names = caps[1].trim().to_string();
            let rest = caps[2].trim().to_string();
            flags.push(Flag {
                names,
                description: rest,
            });
        } else if let Some(last) = flags.last_mut() {
            let t = line.trim();
            // Continuation: indented, non-empty, not a new section header.
            if !t.is_empty() && line.starts_with("  ") && !t.starts_with('-') {
                if !last.description.is_empty() {
                    last.description.push(' ');
                }
                last.description.push_str(t);
            }
        }
    }

    docs.flags = flags;

    // Derive a minimal summary from the command name if we still have none.
    if docs.summary.is_none() {
        docs.summary = Some(format!("{cmd} command"));
    }

    docs
}

fn normalize_usage(cmd: &str, usage: &str) -> String {
    let Some((first, rest)) = usage.split_once(' ') else {
        return usage.to_string();
    };

    let first_name = std::path::Path::new(first)
        .file_name()
        .and_then(|name| name.to_str());

    if first_name == Some(cmd) {
        format!("{cmd} {rest}")
    } else {
        usage.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_typical_help() {
        let text = r"
Usage: cp [OPTION]... [-T] SOURCE DEST

  -r, --recursive   copy directories recursively
  -v, --verbose     explain what is being done
  -n, --no-clobber  do not overwrite an existing file
";
        let docs = parse("cp", text);
        assert!(docs.usage.is_some());
        assert_eq!(docs.flags.len(), 3);
        assert!(docs.flags[0].names.contains("-r"));
    }

    #[test]
    fn parse_no_flags() {
        let text = "A simple command with no documented flags.\n";
        let docs = parse("foo", text);
        assert!(docs.flags.is_empty());
    }

    #[test]
    fn normalizes_usage_from_absolute_exec_path() {
        let text = "Usage: /usr/bin/cp [OPTION]... SOURCE DEST\n";
        let docs = parse("cp", text);
        assert_eq!(docs.usage, Some("cp [OPTION]... SOURCE DEST".to_string()));
    }
}
