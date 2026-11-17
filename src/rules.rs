//! The house rules no linter knows, from the knowledge repos' AGENTS.md:
//!
//! - `frontmatter`: plain Markdown, no YAML frontmatter outside the allowed paths.
//! - `link`, `anchor`: relative links and their `#anchors` resolve, with the exact case,
//!   since GitHub's file names are case-sensitive and the Mac's aren't.
//! - `superseded-date`: a block that starts "Superseded" carries a date.
//! - `undated-now`: "currently" and words like it have a date nearby. A warning, since
//!   plenty of these are fine in context.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use percent_encoding::percent_decode_str;
use regex::Regex;

use crate::config::{Repo, Tool};
use crate::contract::{Finding, Severity};
use crate::generate::{self, SHARED_FRONTMATTER};
use crate::markdown::{self, Block, BlockKind, Lines, Patterns};

/// A date as these repos write them: 2026-10-03, 2026-10, October 3rd, 2026, 3 Oct 2026,
/// Aug 2026, or a bare year from 1990 on.
// cspell:ignore uary ruary ober
pub static DATE: LazyLock<Regex> = LazyLock::new(|| {
    let month = r"(?:Jan(?:uary)?|Feb(?:ruary)?|Mar(?:ch)?|Apr(?:il)?|May|June?|July?|Aug(?:ust)?|Sept?(?:ember)?|Oct(?:ober)?|Nov(?:ember)?|Dec(?:ember)?)";
    Regex::new(&format!(
        r"(?x)
        \b(?:19|20)\d{{2}}-\d{{2}}(?:-\d{{2}})?\b
        | \b{month}\.?\s+\d{{1,2}}(?:st|nd|rd|th)?\b
        | \b\d{{1,2}}(?:st|nd|rd|th)?\s+{month}\b
        | \b{month}\.?\s+(?:19|20)\d{{2}}\b
        | \b(?:199\d|20\d{{2}})\b
        "
    ))
    .unwrap()
});

static NOW_WORDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(currently|at the moment|right now|at present|as of now|nowadays|these days|presently)\b").unwrap()
});

static SUPERSEDED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^superseded\b").unwrap());

static SCHEME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9+.-]*:").unwrap());

pub struct Rules<'a> {
    repo: &'a Repo,
    /// Check links into sibling repos; see `check::Options::siblings`.
    siblings: bool,
    /// Folder listings, for exact-case lookups.
    dirs: HashMap<PathBuf, Option<HashSet<String>>>,
    anchors: HashMap<PathBuf, HashSet<String>>,
}

impl<'a> Rules<'a> {
    pub fn new(repo: &'a Repo, siblings: bool) -> Rules<'a> {
        Rules {
            repo,
            siblings,
            dirs: HashMap::new(),
            anchors: HashMap::new(),
        }
    }

    /// Every Markdown file the rules read, relative to the repo root.
    pub fn files(&self) -> Vec<PathBuf> {
        let skip = generate::ignores(&self.repo.config, Tool::Rules)
            .into_iter()
            .flat_map(|i| i.paths);
        markdown::files(&self.repo.root, &Patterns::new(&self.repo.root, skip))
    }

    pub fn check(&mut self) -> Vec<Finding> {
        let allow = Patterns::new(
            &self.repo.root,
            SHARED_FRONTMATTER
                .iter()
                .map(|s| s.to_string())
                .chain(self.repo.config.frontmatter.allow.iter().cloned()),
        );
        let mut findings = vec![];
        for rel in self.files() {
            let Ok(text) = std::fs::read_to_string(self.repo.root.join(&rel)) else {
                continue;
            };
            let file = rel.to_string_lossy().into_owned();
            let mut add = |line: usize, rule: &str, severity: Severity, message: String| {
                findings.push(Finding {
                    file: Some(file.clone()),
                    line: Some(line),
                    column: None,
                    tool: "stet".into(),
                    rule: rule.into(),
                    severity,
                    message,
                })
            };
            if markdown::frontmatter(&text).is_some() && !allow.matches(&rel) {
                add(
                    1,
                    "frontmatter",
                    Severity::Error,
                    "YAML frontmatter; these repos are plain Markdown (allow a path in .stet.toml)"
                        .into(),
                );
            }
            let lines = Lines::new(&text);
            let doc = markdown::parse(&text);
            for link in doc.links.iter().filter(|l| !l.autolink) {
                if let Some((rule, message)) = self.link(&rel, &text, &link.url) {
                    add(lines.line(link.range.start), rule, Severity::Error, message);
                }
            }
            for (i, block) in doc.blocks.iter().enumerate() {
                let heading = doc.blocks[..i]
                    .iter()
                    .rev()
                    .find(|b| b.kind == BlockKind::Heading && b.start < block.start);
                let before = i.checked_sub(1).map(|j| &doc.blocks[j]);
                let dated = |b: Option<&Block>| b.is_some_and(|b| DATE.is_match(&b.text));
                let prose = block.prose.trim_matches(|c: char| !c.is_alphanumeric());
                // A table cell saying "Superseded" is a status, and a heading saying only
                // that names a section of old versions; neither is a superseded block.
                let label =
                    block.kind == BlockKind::Cell || prose.eq_ignore_ascii_case("superseded");
                if SUPERSEDED.is_match(prose)
                    && !label
                    && !DATE.is_match(&block.text)
                    && !dated(heading)
                    && !dated(before)
                {
                    add(
                        lines.line(block.start),
                        "superseded-date",
                        Severity::Error,
                        "a superseded block needs the date it was superseded".into(),
                    );
                }
                if let Some(m) = NOW_WORDS.find(&block.prose)
                    && !DATE.is_match(&block.text)
                    && !dated(heading)
                    && !dated(before)
                {
                    add(
                        lines.line(block.start),
                        "undated-now",
                        Severity::Warning,
                        format!("\"{}\" with no date nearby", m.as_str()),
                    );
                }
            }
        }
        findings
    }

    /// What's wrong with a link from `from`, if anything.
    fn link(&mut self, from: &Path, text: &str, url: &str) -> Option<(&'static str, String)> {
        let url = url.trim();
        if url.is_empty() || SCHEME.is_match(url) || url.starts_with("//") {
            return None;
        }
        let (path, fragment) = match url.split_once('#') {
            Some((p, f)) => (p, Some(f)),
            None => (url, None),
        };
        let path = percent_decode_str(path).decode_utf8_lossy().into_owned();
        let target = if path.is_empty() {
            from.to_path_buf()
        } else {
            let base = if path.starts_with('/') {
                PathBuf::new()
            } else {
                from.parent().map(Path::to_path_buf).unwrap_or_default()
            };
            match normalize(&base.join(path.trim_start_matches('/'))) {
                Some(p) => p,
                // A sibling repo, read side by side on the Mac. It only has to exist, and
                // is only checked there: CI has this repo alone.
                None if !self.siblings => return None,
                None => {
                    let outside = self.repo.root.join(from).parent()?.join(&path);
                    return (!outside.exists())
                        .then(|| ("link", format!("{url}: no such file outside the repo")));
                }
            }
        };
        if !path.is_empty() {
            match self.exact(&target) {
                Lookup::Found => {}
                Lookup::Missing => return Some(("link", format!("{url}: no such file"))),
                Lookup::Case(actual) => {
                    return Some((
                        "link",
                        format!("{url}: the file is {actual}; case matters on GitHub"),
                    ));
                }
            }
        }
        let fragment = fragment.filter(|f| !f.is_empty())?;
        if target.extension().is_none_or(|e| e != "md") {
            return None;
        }
        let anchors = if path.is_empty() {
            self.anchors
                .entry(target.clone())
                .or_insert_with(|| markdown::anchors(text))
        } else {
            let full = self.repo.root.join(&target);
            self.anchors.entry(target.clone()).or_insert_with(|| {
                markdown::anchors(&std::fs::read_to_string(full).unwrap_or_default())
            })
        };
        let wanted = percent_decode_str(fragment)
            .decode_utf8_lossy()
            .to_lowercase();
        (!anchors.contains(&wanted)).then(|| ("anchor", format!("{url}: no heading #{wanted}")))
    }

    /// Looks a relative path up one component at a time, so a wrong case shows.
    fn exact(&mut self, rel: &Path) -> Lookup {
        let mut dir = self.repo.root.clone();
        let mut fixed = PathBuf::new();
        let mut case = false;
        for component in rel.components() {
            let Component::Normal(name) = component else {
                continue;
            };
            let name = name.to_string_lossy();
            let listing = self
                .dirs
                .entry(dir.clone())
                .or_insert_with(|| list(&dir))
                .as_ref();
            let Some(listing) = listing else {
                return Lookup::Missing;
            };
            let actual = if listing.contains(name.as_ref()) {
                name.into_owned()
            } else if let Some(other) = listing
                .iter()
                .find(|n| n.to_lowercase() == name.to_lowercase())
            {
                case = true;
                other.clone()
            } else {
                return Lookup::Missing;
            };
            dir.push(&actual);
            fixed.push(&actual);
        }
        if case {
            Lookup::Case(fixed.display().to_string())
        } else {
            Lookup::Found
        }
    }
}

enum Lookup {
    Found,
    Missing,
    Case(String),
}

fn list(dir: &Path) -> Option<HashSet<String>> {
    let entries = std::fs::read_dir(dir).ok()?;
    Some(
        entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect(),
    )
}

/// Folds `.` and `..`; None when the path climbs above the root.
fn normalize(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::Normal(n) => out.push(n),
            _ => {}
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_in_house_formats() {
        for text in [
            "on 2026-10-03",
            "2026-10",
            "September 21st, 2026",
            "Aug 2026",
            "3 Oct",
            "since 2024",
            "Oct. 3",
        ] {
            assert!(DATE.is_match(text), "{text}");
        }
        for text in ["version 3.2", "1 of 4", "port 8080", "May I"] {
            assert!(!DATE.is_match(text), "{text}");
        }
    }

    #[test]
    fn normalize_stops_at_the_root() {
        assert_eq!(normalize(Path::new("a/../b")), Some(PathBuf::from("b")));
        assert_eq!(normalize(Path::new("../b")), None);
    }
}
