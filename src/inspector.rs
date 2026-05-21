//! Core inspection logic: resolve command identity and gather trusted docs.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::curated;
use crate::sources::{help, man, ollama, whatis, CommandDocs, Example, Flag, RiskNote};

/// User-selected source policy for an inspection.
#[derive(Debug, Clone, Default)]
pub struct InspectOptions {
    /// Allow running `<command> --help` even when the command is not auto-safe.
    pub allow_help: bool,
    /// Disable all `<command> --help` execution.
    pub no_help: bool,
    /// Call Ollama LLM to enrich output for unknown commands.
    pub use_llm: bool,
    /// Show source/fallback warnings.
    pub verbose: bool,
    /// Optional Ollama model override.
    pub llm_model: Option<String>,
    /// Optional Ollama URL override.
    pub llm_url: Option<String>,
}

/// Full result for `inspect <command>`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandInspection {
    /// Command string as requested by the user.
    pub command: String,
    /// Normalized name used for man/whatis/curated lookup.
    pub doc_command: String,
    /// Local shell/PATH identity.
    pub identity: CommandIdentity,
    /// Short one-line description.
    pub summary: Option<String>,
    /// Concise usage/synopsis.
    pub usage: Option<String>,
    /// Important/common flags.
    pub flags: Vec<Flag>,
    /// Practical examples.
    pub examples: Vec<Example>,
    /// Risk notes and safety reminders.
    pub risks: Vec<RiskNote>,
    /// Sources that contributed data.
    pub sources: Vec<String>,
    /// Non-fatal source warnings.
    pub warnings: Vec<String>,
}

impl CommandInspection {
    pub fn found(&self) -> bool {
        !matches!(self.identity.kind, CommandKind::NotFound)
    }

    fn new(command: &str, doc_command: &str, identity: CommandIdentity) -> Self {
        Self {
            command: command.to_string(),
            doc_command: doc_command.to_string(),
            identity,
            summary: None,
            usage: None,
            flags: Vec::new(),
            examples: Vec::new(),
            risks: Vec::new(),
            sources: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn has_docs(&self) -> bool {
        self.summary.is_some()
            || self.usage.is_some()
            || !self.flags.is_empty()
            || !self.examples.is_empty()
    }
}

/// Local identity for a command.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandIdentity {
    pub kind: CommandKind,
    pub path: Option<String>,
    pub builtin_shell: Option<String>,
}

/// What kind of command was found locally.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandKind {
    External,
    Builtin,
    BuiltinAndExternal,
    NotFound,
}

/// Inspect a bare command name for lookup mode.
pub fn inspect_command(cmd: &str, options: InspectOptions) -> CommandInspection {
    let doc_command = doc_command_name(cmd);
    // For hyphenated git-style names (e.g. "git-commit"), resolve identity
    // against the base tool so the binary is found on PATH.
    let identity_cmd = if let Some(base) = git_style_base(&doc_command) {
        base
    } else {
        cmd
    };
    let identity = resolve_identity(identity_cmd, &doc_command);
    let mut inspection = CommandInspection::new(cmd, &doc_command, identity);

    if let Some(docs) = curated::fetch(&doc_command) {
        merge_docs(&mut inspection, docs, MergePolicy::PreferExisting);
    }

    if let Some(summary) = whatis::fetch(&doc_command) {
        inspection.summary = Some(summary);
        push_source(&mut inspection.sources, "whatis");
    }

    if let Some(docs) = man::fetch(&doc_command) {
        merge_docs(&mut inspection, docs, MergePolicy::PreferIncomingUsage);
    }

    if should_run_help(&doc_command, &inspection.identity, &options) {
        let exec = inspection.identity.path.as_deref().unwrap_or(cmd);
        if let Some(docs) = help::fetch_from(exec, &doc_command, &format!("{doc_command} --help")) {
            merge_docs(&mut inspection, docs, MergePolicy::PreferIncomingUsage);
        }
    }

    if options.use_llm {
        // Collect raw flag text from whatever we have so far.
        let raw_flags: String = inspection
            .flags
            .iter()
            .map(|f| format!("{}: {}", f.names, f.description))
            .collect::<Vec<_>>()
            .join("; ");

        let config =
            ollama::Config::from_options(options.llm_model.as_deref(), options.llm_url.as_deref());
        let show_llm_warning = options.use_llm || options.verbose;
        match ollama::fetch(
            &doc_command,
            inspection.summary.as_deref(),
            &raw_flags,
            &config,
        ) {
            Ok(Some(docs)) => merge_docs(&mut inspection, docs, MergePolicy::PreferIncomingUsage),
            Ok(None) if show_llm_warning => inspection
                .warnings
                .push("ollama returned no usable summary, flags, or examples".to_string()),
            Ok(None) => {}
            Err(error) if show_llm_warning => inspection.warnings.push(format!("ollama: {error}")),
            Err(_) => {}
        }
    }

    inspection
}

/// Backward-compatible doc fetch helper for invocation analysis/tests.
#[allow(dead_code)]
pub fn fetch_docs(cmd: &str) -> Option<CommandDocs> {
    let inspection = inspect_command(
        cmd,
        InspectOptions {
            allow_help: true,
            no_help: false,
            use_llm: false,
            verbose: false,
            llm_model: None,
            llm_url: None,
        },
    );

    if inspection.summary.is_none()
        && inspection.usage.is_none()
        && inspection.flags.is_empty()
        && inspection.examples.is_empty()
    {
        None
    } else {
        Some(CommandDocs {
            summary: inspection.summary,
            usage: inspection.usage,
            flags: inspection.flags,
            examples: inspection.examples,
            risks: inspection.risks,
            sources: inspection.sources,
        })
    }
}

// ── Invocation analysis ──────────────────────────────────────────────────────

/// A flag that appeared in the user's invocation, paired with its meaning.
#[derive(Debug, serde::Serialize)]
pub struct ResolvedFlag {
    /// The flag as typed by the user, e.g. `"-r"`.
    pub typed: String,
    /// Description from docs, or a generic fallback.
    pub description: String,
    /// Whether the flag was found in the docs.
    pub known: bool,
}

/// Full analysis of a command invocation like `cp -rn src/ dest/`.
#[derive(Debug, serde::Serialize)]
pub struct InvocationAnalysis {
    /// The full invocation string as given by the user.
    pub invocation: String,
    /// The base command (first token, or `git-commit` style for subcommands).
    pub command: String,
    /// Each flag resolved against known docs.
    pub resolved_flags: Vec<ResolvedFlag>,
    /// Natural-language summary of what the invocation does.
    pub net_effect: String,
    /// Underlying docs used for resolution (for rendering).
    #[serde(skip)]
    pub docs: Option<CommandDocs>,
}

/// Analyse a full invocation string.
///
/// For git-style tool + subcommand (e.g. `git commit`), tries a hyphenated
/// man page (`git-commit`) first, then falls back to the base command.
pub fn analyse_invocation(query: &str, options: InspectOptions) -> InvocationAnalysis {
    let tokens = tokenize_shell_like(query);
    let base_cmd = tokens.first().map(String::as_str).unwrap_or(query);

    // Determine the documentation target.
    let (display_command, doc_target) = resolve_doc_target(&tokens);

    // Fetch docs for this target.
    let docs = fetch_docs_for_target(base_cmd, &doc_target, options);

    // Extract and resolve flags from the invocation.
    let user_flags = extract_user_flags(&tokens);
    let resolved_flags: Vec<ResolvedFlag> = user_flags
        .iter()
        .map(|flag| {
            let description = docs
                .as_ref()
                .map(|d| resolve_flag(flag, &d.flags))
                .unwrap_or_default();
            let known = !description.is_empty();
            ResolvedFlag {
                typed: flag.clone(),
                description,
                known,
            }
        })
        .collect();

    let net_effect = build_net_effect(&display_command, &resolved_flags);

    InvocationAnalysis {
        invocation: query.to_string(),
        command: display_command,
        resolved_flags,
        net_effect,
        docs,
    }
}

/// Given a token list, return (display_command, doc_target).
///
/// For `git commit` → ("git commit", "git-commit").
/// For `docker run` → ("docker run", "docker-run").
/// For `cp` → ("cp", "cp").
fn resolve_doc_target(tokens: &[String]) -> (String, String) {
    const GIT_STYLE: &[&str] = &["git", "docker", "kubectl", "helm"];

    if tokens.len() >= 2 {
        let base = tokens[0].as_str();
        let sub = tokens[1].as_str();
        if GIT_STYLE.contains(&base) && !sub.starts_with('-') {
            let hyphenated = format!("{base}-{sub}");
            return (format!("{base} {sub}"), hyphenated);
        }
    }

    let base = tokens.first().map(String::as_str).unwrap_or("");
    (base.to_string(), base.to_string())
}

/// Fetch docs for the given target, merging curated + man sources.
///
/// For hyphenated targets like "git-commit": curated wins for any flag it
/// knows, man page fills in the rest.  This keeps clean curated descriptions
/// while still surfacing the long tail of flags from the man page.
fn fetch_docs_for_target(
    base_cmd: &str,
    doc_target: &str,
    options: InspectOptions,
) -> Option<CommandDocs> {
    if doc_target.contains('-') {
        // Start with curated (clean, human-written descriptions).
        let mut merged: Option<CommandDocs> = curated::fetch(doc_target);

        // Merge man page on top — adds flags/examples that curated doesn't have,
        // but curated entries win because merge_command_docs uses PreferExisting.
        if let Some(man_docs) = man::fetch(doc_target) {
            match &mut merged {
                Some(base) => merge_command_docs(base, man_docs),
                None => merged = Some(man_docs),
            }
        }

        if merged.is_some() {
            return merged;
        }

        // Last resort: `git help commit` subprocess.
        let parts: Vec<&str> = doc_target.splitn(2, '-').collect();
        if parts.len() == 2 {
            if let Some(docs) = fetch_subcommand_help(parts[0], parts[1]) {
                return Some(docs);
            }
        }
    }

    // Fall back to inspecting the base command normally.
    let inspection = inspect_command(base_cmd, options);
    if inspection.summary.is_some()
        || !inspection.flags.is_empty()
        || !inspection.examples.is_empty()
    {
        Some(CommandDocs {
            summary: inspection.summary,
            usage: inspection.usage,
            flags: inspection.flags,
            examples: inspection.examples,
            risks: inspection.risks,
            sources: inspection.sources,
        })
    } else {
        None
    }
}

/// Merge `incoming` into `base`, preferring existing entries (curated wins).
///
/// Examples are only taken from `incoming` when `base` has none — curated
/// examples have descriptions and are always preferable to raw man-page ones.
fn merge_command_docs(base: &mut CommandDocs, incoming: CommandDocs) {
    if base.summary.is_none() {
        base.summary = incoming.summary;
    }
    if base.usage.is_none() {
        base.usage = incoming.usage;
    }
    for flag in incoming.flags {
        if !base
            .flags
            .iter()
            .any(|existing| flags_overlap(&existing.names, &flag.names))
        {
            base.flags.push(flag);
        }
    }
    // Only add man-page examples when the curated set is empty — man page
    // EXAMPLES sections for complex tools (git, rsync) often contain prose.
    if base.examples.is_empty() {
        for example in incoming.examples {
            base.examples.push(example);
        }
    }
    for risk in incoming.risks {
        if !base
            .risks
            .iter()
            .any(|existing| existing.message == risk.message)
        {
            base.risks.push(risk);
        }
    }
    for source in incoming.sources {
        if !base.sources.iter().any(|s| s == &source) {
            base.sources.push(source);
        }
    }
}

/// Try `<tool> help <subcommand>` subprocess output (e.g. `git help commit`).
fn fetch_subcommand_help(tool: &str, subcommand: &str) -> Option<CommandDocs> {
    let output = Command::new(tool)
        .args(["help", subcommand])
        .env("GIT_PAGER", "cat")
        .env("PAGER", "cat")
        .output()
        .ok()?;

    let text = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).into_owned()
    } else {
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    if text.trim().is_empty() {
        return None;
    }
    let label = format!("{tool} help {subcommand}");
    Some(help::parse_text(
        &format!("{tool}-{subcommand}"),
        &text,
        &label,
    ))
}

// ── Merge helpers ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum MergePolicy {
    PreferExisting,
    PreferIncomingUsage,
}

fn merge_docs(target: &mut CommandInspection, docs: CommandDocs, policy: MergePolicy) {
    if target.summary.is_none() {
        target.summary = docs.summary;
    }

    match policy {
        MergePolicy::PreferExisting => {
            if target.usage.is_none() {
                target.usage = docs.usage;
            }
        }
        MergePolicy::PreferIncomingUsage => {
            if docs.usage.is_some() {
                target.usage = docs.usage;
            }
        }
    }

    for flag in docs.flags {
        if !target
            .flags
            .iter()
            .any(|existing| flags_overlap(&existing.names, &flag.names))
        {
            target.flags.push(flag);
        }
    }

    for example in docs.examples {
        if !target
            .examples
            .iter()
            .any(|existing| existing.command == example.command)
        {
            target.examples.push(example);
        }
    }

    for risk in docs.risks {
        if !target
            .risks
            .iter()
            .any(|existing| existing.message == risk.message)
        {
            target.risks.push(risk);
        }
    }

    for source in docs.sources {
        push_source(&mut target.sources, &source);
    }
}

fn push_source(sources: &mut Vec<String>, source: &str) {
    if !sources.iter().any(|existing| existing == source) {
        sources.push(source.to_string());
    }
}

fn flags_overlap(left: &str, right: &str) -> bool {
    let left = flag_tokens(left);
    let right = flag_tokens(right);
    left.iter().any(|token| right.contains(token))
}

fn flag_tokens(names: &str) -> Vec<String> {
    names
        .split([',', ' ', '\t'])
        .map(|token| token.trim())
        .filter(|token| token.starts_with('-'))
        .map(|token| {
            token
                .trim_end_matches(|c: char| {
                    matches!(c, ',' | ';' | ':' | ')' | ']') || c.is_ascii_whitespace()
                })
                .to_string()
        })
        .collect()
}

fn should_run_help(
    doc_command: &str,
    identity: &CommandIdentity,
    options: &InspectOptions,
) -> bool {
    if options.no_help {
        return false;
    }

    if options.allow_help {
        return identity.path.is_some();
    }

    is_help_allowlisted(doc_command) && identity.path.as_deref().is_some_and(is_trusted_help_path)
}

fn is_help_allowlisted(cmd: &str) -> bool {
    matches!(
        cmd,
        "cp" | "mv" | "rm" | "ls" | "find" | "grep" | "tar" | "ps" | "kill" | "curl" | "ssh"
    )
}

fn is_trusted_help_path(path: &str) -> bool {
    let path = Path::new(path);
    ["/bin", "/usr/bin", "/usr/local/bin", "/sbin", "/usr/sbin"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn doc_command_name(cmd: &str) -> String {
    if cmd.contains('/') {
        Path::new(cmd)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(cmd)
            .to_string()
    } else {
        cmd.to_string()
    }
}

fn resolve_identity(cmd: &str, doc_command: &str) -> CommandIdentity {
    let explicit_path = cmd.contains('/');
    let path = if explicit_path {
        let candidate = PathBuf::from(cmd);
        if is_executable(&candidate) {
            Some(display_path(candidate))
        } else {
            None
        }
    } else {
        find_on_path(cmd)
    };

    let builtin = !explicit_path && is_shell_builtin(doc_command);

    let kind = match (builtin, path.is_some()) {
        (true, true) => CommandKind::BuiltinAndExternal,
        (true, false) => CommandKind::Builtin,
        (false, true) => CommandKind::External,
        (false, false) => CommandKind::NotFound,
    };

    CommandIdentity {
        kind,
        path,
        builtin_shell: builtin.then(|| "sh/bash/zsh".to_string()),
    }
}

fn find_on_path(cmd: &str) -> Option<String> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(cmd);
        if is_executable(&candidate) {
            return Some(display_path(candidate));
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.is_file()
        && path
            .metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn display_path(path: PathBuf) -> String {
    path.display().to_string()
}

/// If `cmd` looks like `git-commit` / `docker-run` etc., return the base tool name.
fn git_style_base(cmd: &str) -> Option<&str> {
    const GIT_STYLE_BASES: &[&str] = &["git", "docker", "kubectl", "helm"];
    let (base, _sub) = cmd.split_once('-')?;
    GIT_STYLE_BASES.contains(&base).then_some(base)
}

fn is_shell_builtin(cmd: &str) -> bool {
    matches!(
        cmd,
        "alias"
            | "bg"
            | "bind"
            | "break"
            | "builtin"
            | "caller"
            | "cd"
            | "command"
            | "compgen"
            | "complete"
            | "declare"
            | "dirs"
            | "disown"
            | "echo"
            | "enable"
            | "eval"
            | "exec"
            | "exit"
            | "export"
            | "fc"
            | "fg"
            | "getopts"
            | "hash"
            | "help"
            | "history"
            | "jobs"
            | "kill"
            | "let"
            | "local"
            | "logout"
            | "popd"
            | "printf"
            | "pushd"
            | "pwd"
            | "read"
            | "readarray"
            | "readonly"
            | "return"
            | "set"
            | "shift"
            | "shopt"
            | "source"
            | "suspend"
            | "test"
            | "times"
            | "trap"
            | "type"
            | "typeset"
            | "ulimit"
            | "umask"
            | "unalias"
            | "unset"
            | "wait"
    )
}

// ── Flag extraction helpers ──────────────────────────────────────────────────

/// Extract individual flags from a token list (skip the command itself).
///
/// Handles combined short flags (`-rn` → `-r`, `-n`) and long flags.
fn extract_user_flags(tokens: &[String]) -> Vec<String> {
    let mut flags = Vec::new();

    // Skip first token (the command itself); for git-style, also skip the subcommand.
    let skip = if tokens.len() >= 2
        && matches!(tokens[0].as_str(), "git" | "docker" | "kubectl" | "helm")
        && !tokens[1].starts_with('-')
    {
        2
    } else {
        1
    };

    for token in tokens.iter().skip(skip).map(String::as_str) {
        if matches!(token, "--" | "|" | ";" | "&&" | "||" | ">" | ">>" | "<") {
            break;
        }

        if token.starts_with("--") {
            // Long flag, keep as-is (strip possible `=value`).
            let name = token.find('=').map_or(token, |i| &token[..i]);
            flags.push(name.to_string());
        } else if let Some(rest) = token.strip_prefix('-') {
            if rest.is_empty() {
                continue;
            }
            if rest.len() == 1 || rest.chars().all(|ch| ch.is_ascii_digit()) {
                flags.push(token.to_string());
            } else if rest.chars().all(|ch| ch.is_ascii_alphabetic()) {
                // Expand combined flags: -rn → -r, -n
                for ch in rest.chars() {
                    flags.push(format!("-{ch}"));
                }
            } else {
                // Attached value shorthand like -n20, -mfix, -p2222.
                if let Some(ch) = rest.chars().next() {
                    flags.push(format!("-{ch}"));
                }
            }
        }
        // Non-flag arguments (paths, etc.) are ignored.
    }

    flags
}

fn tokenize_shell_like(input: &str) -> Vec<String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Quote {
        None,
        Single,
        Double,
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = Quote::None;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match (quote, ch) {
            (_, '\\') if quote != Quote::Single => escaped = true,
            (Quote::None, '\'') => quote = Quote::Single,
            (Quote::Single, '\'') => quote = Quote::None,
            (Quote::None, '"') => quote = Quote::Double,
            (Quote::Double, '"') => quote = Quote::None,
            (Quote::None, ch) if ch.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if escaped {
        current.push('\\');
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Look up a single flag string against the docs flag list.
fn resolve_flag(flag: &str, doc_flags: &[Flag]) -> String {
    let bare = flag.trim_start_matches('-');

    for df in doc_flags {
        let tokens: Vec<&str> = df
            .names
            .split([',', ' ', '\t'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        for token in tokens {
            let t_bare = token.trim_start_matches('-');
            if t_bare == bare {
                return df.description.trim().to_string();
            }
        }
    }

    String::new()
}

/// Join resolved flag descriptions into a readable "net effect" sentence.
fn build_net_effect(command: &str, resolved: &[ResolvedFlag]) -> String {
    let non_empty: Vec<&str> = resolved
        .iter()
        .filter(|r| !r.description.is_empty())
        .map(|r| r.description.as_str())
        .collect();

    if non_empty.is_empty() {
        return format!("runs {command} with the specified options");
    }

    let parts: Vec<String> = non_empty
        .iter()
        .map(|s| {
            let sentence = s.split('.').next().unwrap_or(s).trim();
            let mut chars = sentence.chars();
            match chars.next() {
                Some(c) => c.to_lowercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();

    format!("{command}: {}", parts.join(", "))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::Flag;

    fn make_docs(flags: Vec<Flag>) -> CommandDocs {
        CommandDocs {
            summary: Some("test command".to_string()),
            flags,
            ..Default::default()
        }
    }

    #[test]
    fn combined_short_flags_expanded() {
        let tokens = vec![
            "cp".to_string(),
            "-rn".to_string(),
            "src/".to_string(),
            "dest/".to_string(),
        ];
        let flags = extract_user_flags(&tokens);
        assert_eq!(flags, vec!["-r", "-n"]);
    }

    #[test]
    fn long_flag_stripped_of_value() {
        let tokens = vec!["cmd".to_string(), "--output=file.txt".to_string()];
        let flags = extract_user_flags(&tokens);
        assert_eq!(flags, vec!["--output"]);
    }

    #[test]
    fn resolve_known_flag() {
        let docs = make_docs(vec![Flag {
            names: "-r, --recursive".to_string(),
            description: "copy directories recursively".to_string(),
        }]);
        let desc = resolve_flag("-r", &docs.flags);
        assert_eq!(desc, "copy directories recursively");
    }

    #[test]
    fn resolve_unknown_flag_returns_empty() {
        let docs = make_docs(vec![]);
        let desc = resolve_flag("-z", &docs.flags);
        assert!(desc.is_empty());
    }

    #[test]
    fn net_effect_joins_descriptions() {
        let resolved = vec![
            ResolvedFlag {
                typed: "-r".to_string(),
                description: "Copy directories recursively".to_string(),
                known: true,
            },
            ResolvedFlag {
                typed: "-n".to_string(),
                description: "Do not overwrite existing files".to_string(),
                known: true,
            },
        ];
        let effect = build_net_effect("cp", &resolved);
        assert!(effect.contains("cp"));
        assert!(effect.contains("copy directories recursively"));
    }

    #[test]
    fn git_style_subcommand_skips_subcommand_token_for_flags() {
        let tokens = vec![
            "git".to_string(),
            "commit".to_string(),
            "-m".to_string(),
            "fix typo".to_string(),
        ];
        let flags = extract_user_flags(&tokens);
        assert_eq!(flags, vec!["-m"]);
    }

    #[test]
    fn resolve_doc_target_git_commit() {
        let tokens = vec![
            "git".to_string(),
            "commit".to_string(),
            "-m".to_string(),
            "msg".to_string(),
        ];
        let (display, target) = resolve_doc_target(&tokens);
        assert_eq!(display, "git commit");
        assert_eq!(target, "git-commit");
    }

    #[test]
    fn resolve_doc_target_plain_command() {
        let tokens = vec![
            "cp".to_string(),
            "-r".to_string(),
            "src/".to_string(),
            "dest/".to_string(),
        ];
        let (display, target) = resolve_doc_target(&tokens);
        assert_eq!(display, "cp");
        assert_eq!(target, "cp");
    }

    #[test]
    fn tokenizer_preserves_quoted_arguments() {
        let tokens = tokenize_shell_like(r#"git commit -m "fix typo""#);
        assert_eq!(tokens, vec!["git", "commit", "-m", "fix typo"]);
    }

    #[test]
    fn flag_extraction_handles_numeric_and_attached_values() {
        let numeric = vec!["head".to_string(), "-20".to_string(), "file".to_string()];
        assert_eq!(extract_user_flags(&numeric), vec!["-20"]);

        let attached = vec!["head".to_string(), "-n20".to_string(), "file".to_string()];
        assert_eq!(extract_user_flags(&attached), vec!["-n"]);
    }

    #[test]
    fn flag_extraction_stops_at_double_dash_and_pipes() {
        let double_dash = vec![
            "cmd".to_string(),
            "-a".to_string(),
            "--".to_string(),
            "--literal".to_string(),
        ];
        assert_eq!(extract_user_flags(&double_dash), vec!["-a"]);

        let pipe = vec![
            "grep".to_string(),
            "-n".to_string(),
            "foo".to_string(),
            "|".to_string(),
            "wc".to_string(),
            "-l".to_string(),
        ];
        assert_eq!(extract_user_flags(&pipe), vec!["-n"]);
    }
}
