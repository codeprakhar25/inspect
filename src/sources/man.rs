//! Fetch and parse `man` pages.

use regex::Regex;
use std::process::Command;

use super::{CommandDocs, Example, Flag};

/// Run `man -P cat <cmd>` and parse the output into [`CommandDocs`].
///
/// Returns `None` when `man` is unavailable or the page does not exist.
///
pub fn fetch(cmd: &str) -> Option<CommandDocs> {
    let output = Command::new("man")
        .args(["-P", "cat", cmd])
        .env("MANWIDTH", "80")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let clean = strip_formatting(&raw);

    let mut docs = parse(&clean);
    docs.sources.push("man".to_string());
    Some(docs)
}

/// Remove ANSI escape sequences and overstrike bold (`x\x08x`) from man output.
fn strip_formatting(input: &str) -> String {
    // Remove overstrike bold: char + backspace + char  -> char
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 2 < bytes.len() && bytes[i + 1] == b'\x08' {
            // Skip the first char and the backspace; keep the second char.
            i += 2;
            continue;
        }
        result.push(bytes[i] as char);
        i += 1;
    }

    // Remove ANSI escape codes  ESC [ ... m
    let ansi_re = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").expect("static regex is valid");
    normalize_encoding_artifacts(&ansi_re.replace_all(&result, ""))
}

fn normalize_encoding_artifacts(input: &str) -> String {
    let normalized = input
        .replace("â", "-")
        .replace("â", "-")
        .replace("â", "-")
        .replace("â", "-")
        .replace("â", "-")
        .replace(
            ['\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}'],
            "-",
        )
        .replace("â", "'")
        .replace("â", "'")
        .replace("â", "\"")
        .replace("â", "\"");

    let soft_wrap_re =
        Regex::new(r"([A-Za-z]{2,})- ([A-Za-z]{2,})").expect("static regex is valid");
    soft_wrap_re.replace_all(&normalized, "$1$2").into_owned()
}

/// Parse a clean (no formatting) man page text into [`CommandDocs`].
fn parse(text: &str) -> CommandDocs {
    let mut docs = CommandDocs::default();

    let section_re = Regex::new(r"^[A-Z][A-Z0-9 _-]+$").expect("static regex is valid");

    // Collect section contents keyed by heading.
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_end();
        if section_re.is_match(trimmed.trim()) && !trimmed.trim().is_empty() {
            sections.push((trimmed.trim().to_string(), Vec::new()));
        } else if let Some((_, lines)) = sections.last_mut() {
            lines.push(trimmed.to_string());
        }
    }

    for (heading, lines) in &sections {
        match heading.as_str() {
            "NAME" => {
                // "cp - copy files or directories"
                let joined = lines.iter().map(|l| l.trim()).collect::<Vec<_>>().join(" ");
                if let Some(dash_pos) = joined.find(" - ") {
                    docs.summary = Some(joined[dash_pos + 3..].trim().to_string());
                } else if let Some(dash_pos) = joined.find(" \u{2014} ") {
                    docs.summary = Some(joined[dash_pos + 4..].trim().to_string());
                } else if !joined.trim().is_empty() {
                    docs.summary = Some(joined.trim().to_string());
                }
            }
            "SYNOPSIS" | "USAGE" => {
                // Only take the first non-empty line — full SYNOPSIS can be huge.
                let first = lines.iter().find(|l| !l.trim().is_empty());
                if let Some(line) = first {
                    docs.usage = Some(line.trim().to_string());
                }
            }
            "DESCRIPTION" => {
                // First non-empty line as a fallback summary if NAME didn't give us one.
                if docs.summary.is_none() {
                    let first = lines.iter().find(|l| !l.trim().is_empty());
                    if let Some(line) = first {
                        docs.summary = Some(line.trim().to_string());
                    }
                }
                // Many GNU tools (cp, mv, ls…) embed their flags in DESCRIPTION
                // rather than a separate OPTIONS section. Parse them here; the
                // OPTIONS branch below will overwrite if a dedicated section exists.
                let desc_flags = parse_flag_lines(lines);
                if !desc_flags.is_empty() {
                    docs.flags = desc_flags;
                }
            }
            "OPTIONS" => {
                // Dedicated OPTIONS section takes priority over DESCRIPTION flags.
                docs.flags = parse_flag_lines(lines);
            }
            "EXAMPLES" | "EXAMPLE" => {
                for line in lines {
                    let t = line.trim();
                    // Only include lines that look like shell commands, not prose.
                    if !t.is_empty() && looks_like_command(t) && !has_excessive_spaces(t) {
                        docs.examples.push(Example {
                            command: t.to_string(),
                            description: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    docs
}

/// Parse a block of lines from the OPTIONS/DESCRIPTION section into [`Flag`] entries.
///
/// Handles two GNU man-page styles:
///   -r, --recursive          ← names-only line, description on next line
///   -b     like --backup…    ← names + inline description on same line
fn parse_flag_lines(lines: &[String]) -> Vec<Flag> {
    // Match: moderate indent, then one or more comma-separated flag tokens.
    // Group 1: all flag tokens (e.g. "-R, -r, --recursive")
    // Group 2: anything after the last flag token on the same line
    let flag_line_re = Regex::new(r"^\s{1,12}((?:-[\w-]+)(?:,\s*-[\w-]+|\s+-[\w-]+)*)(.*)$")
        .expect("static regex is valid");

    let mut flags: Vec<Flag> = Vec::new();

    for line in lines {
        if let Some(caps) = flag_line_re.captures(line) {
            let names = caps[1].trim().to_string();
            let rest = caps[2].trim().to_string();
            // Strip a leading `=VALUE` or `[=VALUE]` metavar from rest.
            let description = strip_metavar(&rest);
            flags.push(Flag { names, description });
        } else if let Some(last) = flags.last_mut() {
            // Continuation line — append to previous description.
            let t = line.trim();
            if !t.is_empty() && leading_whitespace(line) >= 10 {
                if last.description.is_empty() {
                    last.description = t.to_string();
                } else if ends_with_hyphenish(&last.description)
                    && t.chars().next().is_some_and(char::is_alphabetic)
                {
                    last.description.pop();
                    last.description.push_str(t);
                } else {
                    last.description.push(' ');
                    last.description.push_str(t);
                }
            }
        }
    }

    flags
}

fn leading_whitespace(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

fn ends_with_hyphenish(text: &str) -> bool {
    text.chars().last().is_some_and(|ch| {
        matches!(
            ch,
            '-' | '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}'
        )
    })
}

/// Common English words that begin prose sentences but are never command names.
const PROSE_STARTERS: &[&str] = &[
    "a",
    "an",
    "the",
    "this",
    "that",
    "these",
    "those",
    "it",
    "if",
    "in",
    "on",
    "at",
    "to",
    "for",
    "from",
    "with",
    "without",
    "not",
    "and",
    "or",
    "but",
    "nor",
    "so",
    "yet",
    "both",
    "when",
    "while",
    "since",
    "until",
    "unless",
    "because",
    "although",
    "though",
    "whether",
    "after",
    "before",
    "as",
    "by",
    "which",
    "however",
    "note",
    "see",
    "you",
    "we",
    "here",
    "there",
    "each",
    "any",
    "all",
    "some",
    "such",
    "your",
    "our",
    "their",
    "its",
    "then",
    "than",
    "working",
    "staging",
    "modified",
    "committed",
    "using",
    "given",
    "each",
];

/// Heuristic: return true if a trimmed line looks like a shell command rather
/// than prose.  Filters lines that begin with uppercase (English sentences)
/// and lines whose first word is a common prose function-word or continuation
/// fragment.
/// Return true if a line has 3+ consecutive spaces, which typically indicates
/// man-page justification/padding in prose paragraphs rather than a command.
fn has_excessive_spaces(s: &str) -> bool {
    s.contains("   ")
}

fn looks_like_command(s: &str) -> bool {
    let first = match s.chars().next() {
        Some(c) => c,
        None => return false,
    };

    // Reject lines starting with uppercase (prose sentences, section headings).
    if first.is_ascii_uppercase() {
        return false;
    }

    // Always allow obvious shell starters.
    if matches!(first, '$' | '/' | '.') || first.is_ascii_digit() {
        return true;
    }

    // For lowercase-starting lines, reject common English prose starters.
    if first.is_ascii_lowercase() {
        let first_word: &str = s.split_whitespace().next().unwrap_or("");
        let first_word_lower = first_word.to_ascii_lowercase();
        // Strip trailing punctuation for comparison.
        let bare = first_word_lower.trim_end_matches(|c: char| !c.is_alphabetic());
        if PROSE_STARTERS.contains(&bare) {
            return false;
        }
        return true;
    }

    false
}

/// Remove a leading `[=VALUE]` or `=VALUE` metavar from a flag's trailing text,
/// leaving only the human-readable description.
fn strip_metavar(rest: &str) -> String {
    let s = rest.trim();
    // Strip optional `[=WORD]` or `=WORD`
    let s = if s.starts_with("[=") || s.starts_with('=') {
        let end = s.find(|c: char| c.is_whitespace()).unwrap_or(s.len());
        s[end..].trim()
    } else {
        s
    };
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_overstrike_bold() {
        let input = "c\x08cp\x08p";
        assert_eq!(strip_formatting(input), "cp");
    }

    #[test]
    fn strip_ansi() {
        let input = "\x1b[1mcp\x1b[0m";
        assert_eq!(strip_formatting(input), "cp");
    }

    #[test]
    fn normalizes_common_mojibake_punctuation() {
        assert_eq!(normalize_encoding_artifacts("UPâDATE"), "UP-DATE");
    }

    #[test]
    fn joins_soft_hyphen_line_wraps() {
        assert_eq!(normalize_encoding_artifacts("UPâ DATE"), "UPDATE");
        assert_eq!(normalize_encoding_artifacts("UP\u{2010} DATE"), "UPDATE");
        assert_eq!(normalize_encoding_artifacts("reâ placed"), "replaced");
    }

    #[test]
    fn parse_flag_block() {
        let lines: Vec<String> = vec![
            "       -r, --recursive".to_string(),
            "              copy directories recursively".to_string(),
            "       -v, --verbose".to_string(),
            "              explain what is being done".to_string(),
        ];
        let flags = parse_flag_lines(&lines);
        assert_eq!(flags.len(), 2);
        assert!(flags[0].names.contains("-r"));
        assert!(!flags[0].description.is_empty());
    }

    #[test]
    fn stops_flag_continuation_at_body_paragraphs() {
        let lines: Vec<String> = vec![
            "       --version".to_string(),
            "              output version information and exit".to_string(),
            "".to_string(),
            "       ATTR_LIST is a comma-separated list of attributes.".to_string(),
        ];
        let flags = parse_flag_lines(&lines);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].description, "output version information and exit");
    }

    #[test]
    fn joins_wrapped_flag_descriptions_without_soft_hyphen() {
        let lines: Vec<String> = vec![
            "       --update[=UPDATE]".to_string(),
            "              control which files are updated; UP‐".to_string(),
            "              DATE={all,none,older(default)}. See below".to_string(),
        ];
        let flags = parse_flag_lines(&lines);
        assert_eq!(
            flags[0].description,
            "control which files are updated; UPDATE={all,none,older(default)}. See below"
        );
    }
}
