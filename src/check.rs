//! `stet check`: configs in sync, then Prettier, cspell, markdownlint and the repo's own
//! check tasks one after another, then the house rules.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use miette::{IntoDiagnostic, Result, miette};

use crate::config::Repo;
use crate::contract::{Finding, RepoCheck, Severity};
use crate::rules::Rules;
use crate::sync;
use crate::tools::Tools;

#[derive(Debug, Clone)]
pub struct Options {
    /// Let Prettier rewrite files. Off, it only checks, as CI does.
    pub write: bool,
    /// Run the Node tools and the repo's tasks. Off, only the config and house rules.
    pub tools: bool,
    /// Keep only findings in these repo-relative files (`--files`). Findings with no file,
    /// like a tool that failed to run, and config drift are always kept.
    pub only: Option<HashSet<String>>,
}

/// `--files` paths as the tools report them: relative to the repo root, with `/`.
pub fn relative(root: &Path, files: &[PathBuf]) -> Result<HashSet<String>> {
    let cwd = std::env::current_dir().into_diagnostic()?;
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    files
        .iter()
        .map(|f| {
            let path = std::path::absolute(cwd.join(f)).into_diagnostic()?;
            // A deleted file can't be canonicalized; its folder usually can.
            let path = match (path.parent(), path.file_name()) {
                (Some(dir), Some(name)) => dir
                    .canonicalize()
                    .map(|d| d.join(name))
                    .unwrap_or(path.clone()),
                _ => path.clone(),
            };
            let rel = path
                .strip_prefix(&root)
                .map_err(|_| miette!("{} isn't in {}", f.display(), root.display()))?;
            Ok(rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/"))
        })
        .collect()
}

pub fn check(repo: &Repo, tools: &Tools, options: &Options) -> RepoCheck {
    let mut findings: Vec<Finding> = sync::drifted(repo)
        .into_iter()
        .map(|path| Finding {
            file: Some(path.into()),
            line: None,
            column: None,
            tool: "stet".into(),
            rule: "config-sync".into(),
            severity: Severity::Error,
            message: "differs from what .stet.toml generates; run `stet sync`".into(),
        })
        .chain(sync::leftover(repo).into_iter().map(|path| {
            Finding {
                file: Some(path.into()),
                line: None,
                column: None,
                tool: "stet".into(),
                rule: "config-sync".into(),
                severity: Severity::Error,
                message: "left by an older stet, which kept tool configs in the repo; \
                      run `stet sync` to remove it"
                    .into(),
            }
        }))
        .collect();
    let mut runs = vec![];
    let mut formatted = vec![];
    if options.tools {
        let mut outcomes = vec![
            tools.prettier(repo, options.write),
            tools.cspell(repo),
            tools.markdownlint(repo),
        ];
        for task in &repo.config.mise.check_tasks {
            outcomes.push(tools.task(&repo.root, task));
        }
        for outcome in outcomes {
            runs.push(outcome.run);
            findings.extend(outcome.findings);
            formatted.extend(outcome.formatted);
        }
    }
    findings.extend(Rules::new(repo).check());
    if let Some(only) = &options.only {
        findings.retain(|f| {
            f.rule == "config-sync" || f.file.as_ref().is_none_or(|file| only.contains(file))
        });
    }
    findings.sort_by(|a, b| {
        (a.severity, &a.file, a.line, &a.rule).cmp(&(b.severity, &b.file, b.line, &b.rule))
    });
    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    RepoCheck {
        name: repo.name(),
        path: repo.root.display().to_string(),
        ok: errors == 0,
        errors,
        warnings: findings.len() - errors,
        formatted,
        tools: runs,
        findings,
    }
}
