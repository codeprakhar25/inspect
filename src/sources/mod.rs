pub mod help;
pub mod man;
pub mod ollama;
pub mod tldr;
pub mod whatis;

/// A parsed flag entry from any documentation source.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Flag {
    /// The flag text(s), e.g. `"-r"` or `"-r, --recursive"`.
    pub names: String,
    /// One-line description of what the flag does.
    pub description: String,
}

/// A practical command example.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Example {
    /// Complete example invocation.
    pub command: String,
    /// Short explanation of when/why to use it.
    pub description: Option<String>,
}

/// Risk severity for a command note.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Info,
    Caution,
    Danger,
}

/// A warning or practical caution attached to a command.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RiskNote {
    pub level: RiskLevel,
    pub message: String,
}

/// Aggregated documentation for a command.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandDocs {
    /// Short one-line description of the command.
    pub summary: Option<String>,
    /// Inferred usage line.
    pub usage: Option<String>,
    /// All flags found.
    pub flags: Vec<Flag>,
    /// Example invocations found.
    pub examples: Vec<Example>,
    /// Risk notes and safety reminders.
    pub risks: Vec<RiskNote>,
    /// Sources that supplied data.
    pub sources: Vec<String>,
}
