//! The shape of every `stet … --json` document. Data only: no file access, no logic.
//! `Docs/json-contract.md` describes the same shapes for readers who can't read Rust;
//! change both in the same commit.

use serde::Serialize;

/// The contract version. Every document carries it; bump it on any breaking change.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// One problem in one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Relative to the repo root. Absent for a problem with no file, like a tool that
    /// wouldn't start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// `stet`, `prettier`, `cspell`, `markdownlint` or `mise`.
    pub tool: String,
    pub rule: String,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRun {
    pub name: String,
    pub ok: bool,
    /// How many files the tool read, when it says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoCheck {
    pub name: String,
    pub path: String,
    pub ok: bool,
    pub errors: usize,
    pub warnings: usize,
    /// Files Prettier reformatted in place.
    pub formatted: Vec<String>,
    pub tools: Vec<ToolRun>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckReport {
    pub schema: u32,
    pub ok: bool,
    pub errors: usize,
    pub warnings: usize,
    pub repos: Vec<RepoCheck>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncAction {
    Written,
    Unchanged,
    /// A tool config an older stet generated; stet keeps it in its cache now.
    Removed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncFile {
    pub path: String,
    pub action: SyncAction,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSync {
    pub name: String,
    pub path: String,
    pub files: Vec<SyncFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub schema: u32,
    pub repos: Vec<RepoSync>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemKind {
    File,
    Folder,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
    pub name: String,
    pub path: String,
    pub kind: ItemKind,
    /// Bytes; for a folder, everything inside it.
    pub bytes: u64,
    /// ISO 8601 in UTC, to the second.
    pub modified: String,
}

/// A repo's "Mail that belongs here" section, as data.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailRules {
    /// The section's bullets: what mail goes to this repo.
    pub belongs: Vec<String>,
    /// The "Not here:" paragraph, without the label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_here: Option<String>,
    /// Any other paragraph after the bullets, like how to word a question email.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInbox {
    pub name: String,
    pub path: String,
    pub inbox: String,
    pub items: Vec<InboxItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mail: Option<MailRules>,
    /// Why `mail` or `items` is missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxReport {
    pub schema: u32,
    /// Items waiting across every repo.
    pub waiting: usize,
    pub repos: Vec<RepoInbox>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitReport {
    pub schema: u32,
    pub path: String,
    pub name: String,
    /// Every file and folder created, relative to `path`.
    pub created: Vec<String>,
    /// What's left to do by hand.
    pub next: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkAction {
    /// The link became its text.
    Unlinked,
    /// The link, an image or a reference definition, was dropped.
    Removed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewrittenLink {
    /// Line in the source note.
    pub line: usize,
    pub url: String,
    pub action: LinkAction,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishReport {
    pub schema: u32,
    pub source: String,
    pub target: String,
    pub title: String,
    pub slug: String,
    pub links: Vec<RewrittenLink>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckStatus {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorCheck {
    pub id: String,
    pub status: CheckStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub schema: u32,
    pub ok: bool,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorReport {
    pub schema: u32,
    pub error: String,
    /// Each reason, when there's more than one, like everything `publish` refused over.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}
