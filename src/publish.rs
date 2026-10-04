//! `stet publish <note>`: copies one note into the site repo as an article draft. It
//! refuses a note that's flagged for review, looks like it holds a secret, or comes from a
//! repo with `publish = false`, and turns links into the private repos into plain text,
//! since they'd break on a public site. It writes one file and never commits or deploys.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use miette::{IntoDiagnostic, Result, WrapErr, miette};
use regex::Regex;

use crate::config::{Global, Repo};
use crate::contract::{LinkAction, PublishReport, RewrittenLink, SCHEMA};
use crate::env::Env;
use crate::markdown::{self, Lines};

/// Why a note can't be published. Each reason names a line, never the secret itself.
#[derive(Debug)]
pub struct Refused {
    pub note: String,
    pub reasons: Vec<String>,
}

pub enum Outcome {
    Published(PublishReport),
    Refused(Refused),
}

pub struct Request<'a> {
    pub note: &'a Path,
    pub slug: Option<&'a str>,
    pub force: bool,
}

/// Conservative patterns for things on the never-list. A false alarm costs an edit; a
/// miss costs a rotated key and a public page.
static SECRETS: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    [
        ("a private key", r"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
        ("an AWS key", r"\bAKIA[0-9A-Z]{16}\b"),
        ("a GitHub token", r"\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{20,})"),
        ("a Slack token", r"\bxox[abposr]-[A-Za-z0-9-]{10,}"),
        ("an API key", r"\bsk-(?:ant-|proj-)?[A-Za-z0-9_-]{20,}"),
        ("a Google API key", r"\bAIza[0-9A-Za-z_-]{35}\b"),
        ("a Stripe key", r"\b[rs]k_live_[0-9A-Za-z]{16,}"),
        ("a JSON web token", r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}"),
        ("a 2FA seed", r"(?i)otpauth://"),
        (
            "a password or key",
            r"(?i)\b(?:password|passwd|passcode|api[_ -]?key|secret|token|pin)\b\s*[:=]\s*(\S{4,})",
        ),
        (
            "an account number",
            r"(?i)\b(?:account|acct|card|transit|routing|iban|sin|policy)\b(?:\s+(?:number|no\.?|#))?\s*[:#]?\s*\d[\d -]{5,}\d",
        ),
    ]
    .into_iter()
    .map(|(kind, re)| (kind, Regex::new(re).unwrap()))
    .collect()
});

static ENV_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^`?\$?[A-Z][A-Z0-9_]*`?[.,;:)]?$").unwrap());

static CARD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:\d[ -]?){12,18}\d\b").unwrap());

static REVIEW: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)publish[- ]review").unwrap());

static SCHEME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9+.-]*:").unwrap());

static GITHUB: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)github\.com[:/]([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+?)(?:\.git)?(?:[/#?]|$)")
        .unwrap()
});

static REFERENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^ {0,3}\[[^\]]+\]:[ \t]*<?([^\s>]+)>?.*\n?").unwrap());

static DATE_PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}-").unwrap());

pub fn publish(env: &Env, global: &Global, request: &Request) -> Result<Outcome> {
    let note = std::path::absolute(request.note).into_diagnostic()?;
    let shown = global.abbreviate(&note);
    let text = std::fs::read_to_string(&note)
        .into_diagnostic()
        .wrap_err_with(|| format!("can't read {shown}"))?;
    let repo = Repo::find(&note)?;
    let refuse = |reasons: Vec<String>| {
        Ok(Outcome::Refused(Refused {
            note: shown.clone(),
            reasons,
        }))
    };
    if !repo.config.publish {
        return refuse(vec![format!(
            "{} notes are never published (publish = false in its .stet.toml)",
            repo.name()
        )]);
    }
    let rel = note.strip_prefix(&repo.root).unwrap_or(&note);
    if rel.starts_with("Inbox") {
        return refuse(vec!["it's still in Inbox/; file it first".into()]);
    }
    let never = markdown::Patterns::new(&repo.root, repo.config.never_publish.iter().cloned());
    if never.matches(rel) {
        return refuse(vec![format!(
            "{} is under never_publish in {}'s .stet.toml",
            rel.display(),
            repo.name()
        )]);
    }
    let reasons = scan(&text);
    if !reasons.is_empty() {
        return refuse(reasons);
    }
    let site = global
        .publish
        .as_ref()
        .ok_or_else(|| miette!("no [publish] site in {}", global.abbreviate(&global.file())))?;

    let (title, body) = split_title(&text, &note);
    let private = private_repos(env, global, &repo);
    let (body, links) = rewrite(&body, &private, &text);
    let slug = match request.slug {
        Some(s) => slugify(s),
        None => slugify(&note.file_stem().unwrap_or_default().to_string_lossy()),
    };
    if slug.is_empty() {
        return Err(miette!(
            "can't make a file name from the note's; pass --slug"
        ));
    }
    let target = site.articles.join(format!("{slug}.md"));
    if target.exists() && !request.force {
        return Err(miette!(
            "{} exists; --force replaces it",
            global.abbreviate(&target)
        ));
    }
    let page = format!(
        "---\ntitle: {}\ndate: {}\ndraft: true\n---\n\n{}",
        quote(&title),
        today(env)?,
        body.trim_start()
    );
    std::fs::create_dir_all(&site.articles).into_diagnostic()?;
    std::fs::write(&target, page)
        .into_diagnostic()
        .wrap_err_with(|| format!("can't write {}", target.display()))?;
    Ok(Outcome::Published(PublishReport {
        schema: SCHEMA,
        source: note.display().to_string(),
        target: target.display().to_string(),
        title,
        slug,
        links,
    }))
}

/// Every reason to refuse the note's text, by line.
pub fn scan(text: &str) -> Vec<String> {
    let mut reasons = vec![];
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if REVIEW.is_match(line) {
            reasons.push(format!("line {n}: flagged ⚠️ publish-review"));
        }
        for (kind, re) in SECRETS.iter() {
            let hit = re.captures_iter(line).any(|c| {
                // `SOPS_AGE_KEY` names a secret; it isn't one.
                c.get(1).is_none_or(|v| !ENV_NAME.is_match(v.as_str()))
            });
            if hit {
                reasons.push(format!("line {n}: looks like {kind}"));
            }
        }
        if CARD.find_iter(line).any(|m| luhn(m.as_str())) {
            reasons.push(format!("line {n}: looks like a card number"));
        }
    }
    reasons
}

fn luhn(text: &str) -> bool {
    let digits: Vec<u32> = text.chars().filter_map(|c| c.to_digit(10)).collect();
    if !(13..=19).contains(&digits.len()) || digits.iter().all(|&d| d == digits[0]) {
        return false;
    }
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let x = d * 2;
                if x > 9 { x - 9 } else { x }
            } else {
                d
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

/// The title and the body without it: from frontmatter's `title:`, else the first `# `
/// heading, else the file name.
fn split_title(text: &str, note: &Path) -> (String, String) {
    let mut body = text.to_string();
    let mut title = None;
    if let Some(lines) = markdown::frontmatter(text) {
        let block: Vec<&str> = text.lines().take(lines).collect();
        title = block.iter().find_map(|l| {
            l.strip_prefix("title:")
                .map(|t| t.trim().trim_matches(['"', '\'']).to_string())
        });
        body = text.lines().skip(lines).collect::<Vec<_>>().join("\n") + "\n";
    }
    if title.is_none() {
        let mut lines: Vec<&str> = body.lines().collect();
        if let Some(i) = lines.iter().position(|l| !l.trim().is_empty())
            && let Some(h) = lines[i].strip_prefix("# ")
        {
            title = Some(h.trim().to_string());
            lines.remove(i);
            body = lines.join("\n") + "\n";
        }
    }
    let title = title.unwrap_or_else(|| {
        note.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .replace(['-', '_'], " ")
    });
    (title, body)
}

/// `owner/name` of every configured repo's GitHub remote, lowercased, plus the note's own.
fn private_repos(env: &Env, global: &Global, repo: &Repo) -> Vec<String> {
    let mut roots: Vec<PathBuf> = global.repos.clone();
    roots.push(repo.root.clone());
    let git = env.get("STET_GIT").unwrap_or("git");
    let mut out = vec![];
    for root in roots {
        let Ok(output) = Command::new(git)
            .args(["remote", "get-url", "origin"])
            .current_dir(&root)
            .env_clear()
            .envs(env.vars())
            .output()
        else {
            continue;
        };
        let url = String::from_utf8_lossy(&output.stdout);
        if let Some(c) = GITHUB.captures(url.trim()) {
            out.push(format!("{}/{}", &c[1], &c[2]).to_lowercase());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn is_private(url: &str, private: &[String]) -> bool {
    if url.starts_with('#') {
        return false;
    }
    if !SCHEME.is_match(url) && !url.starts_with("//") {
        return true;
    }
    if url.to_lowercase().starts_with("file:") {
        return true;
    }
    GITHUB
        .captures(url)
        .is_some_and(|c| private.contains(&format!("{}/{}", &c[1], &c[2]).to_lowercase()))
}

/// Turns links into the private repos into their text, and drops images and reference
/// definitions that point there. Repeats until nothing changes, for links inside links.
fn rewrite(body: &str, private: &[String], original: &str) -> (String, Vec<RewrittenLink>) {
    let lines = Lines::new(original);
    let mut text = body.to_string();
    let mut log = vec![];
    for _ in 0..4 {
        let doc = markdown::parse(&text);
        let mut edits: Vec<(std::ops::Range<usize>, String, String, LinkAction)> = vec![];
        let mut last_end = 0;
        for link in &doc.links {
            if link.range.start < last_end || !is_private(&link.url, private) {
                continue;
            }
            last_end = link.range.end;
            let inner = link
                .inner
                .clone()
                .map(|r| text[r].to_string())
                .unwrap_or_default();
            let (replacement, action) = if link.image || link.autolink {
                (String::new(), LinkAction::Removed)
            } else {
                (inner, LinkAction::Unlinked)
            };
            edits.push((link.range.clone(), replacement, link.url.clone(), action));
        }
        for c in REFERENCE.captures_iter(&text) {
            let whole = c.get(0).unwrap();
            if is_private(&c[1], private) {
                edits.push((
                    whole.range(),
                    String::new(),
                    c[1].to_string(),
                    LinkAction::Removed,
                ));
            }
        }
        if edits.is_empty() {
            break;
        }
        edits.sort_by_key(|e| std::cmp::Reverse(e.0.start));
        for (range, replacement, url, action) in edits {
            let source = &text[range.clone()];
            let line = original.find(source).map(|at| lines.line(at)).unwrap_or(0);
            log.push(RewrittenLink { line, url, action });
            text.replace_range(range, &replacement);
        }
    }
    log.sort_by_key(|l| l.line);
    (text, log)
}

pub fn slugify(text: &str) -> String {
    let text = DATE_PREFIX.replace(text, "");
    let mut out = String::new();
    for c in text.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn today(env: &Env) -> Result<String> {
    if let Some(day) = env.get("STET_TODAY") {
        return Ok(day.to_string());
    }
    Ok(jiff::Zoned::now().date().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_for_secrets() {
        assert!(scan("Use the API normally.\nNo secrets here: 2026-10-04.\n").is_empty());
        assert!(scan("the only secret: `SOPS_AGE_KEY`. It starts\n").is_empty());
        let text = "key AKIAABCDEFGHIJKLMNOP\npassword: hunter22\nCard 4111 1111 1111 1111\n\
                    ⚠️ publish-review: garage codes\naccount # 12345678\n";
        let reasons = scan(text);
        assert_eq!(
            reasons,
            [
                "line 1: looks like an AWS key",
                "line 2: looks like a password or key",
                "line 3: looks like an account number",
                "line 3: looks like a card number",
                "line 4: flagged ⚠️ publish-review",
                "line 5: looks like an account number",
            ]
        );
    }

    #[test]
    fn luhn_checks() {
        assert!(luhn("4111 1111 1111 1111"));
        assert!(!luhn("4111 1111 1111 1112"));
        assert!(!luhn("0000 0000 0000 0000"));
    }

    #[test]
    fn rewrites_private_links_only() {
        let private = vec!["gshaw/life".to_string()];
        let body = "See [the note](../Life/x.md) and [Rust](https://rust-lang.org).\n\
                    ![photo](img.png) and [repo](https://github.com/gshaw/Life/blob/main/a.md).\n\
                    [up](#top) and [ref][r].\n\n[r]: other.md\n";
        let (out, log) = rewrite(body, &private, body);
        assert_eq!(
            out,
            "See the note and [Rust](https://rust-lang.org).\n and repo.\n[up](#top) and ref.\n\n"
        );
        assert_eq!(log.len(), 5);
    }

    #[test]
    fn titles_and_slugs() {
        let (title, body) = split_title("# Hello there\n\nBody.\n", Path::new("x.md"));
        assert_eq!(
            (title.as_str(), body.as_str()),
            ("Hello there", "\nBody.\n")
        );
        let (title, body) = split_title("---\ntitle: Where?\n---\n\nText\n", Path::new("x.md"));
        assert_eq!((title.as_str(), body.as_str()), ("Where?", "\nText\n"));
        assert_eq!(
            slugify("2026-08-24-where-llms-came-from"),
            "where-llms-came-from"
        );
        assert_eq!(slugify("Web App Recovery"), "web-app-recovery");
    }
}
