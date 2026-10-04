//! `stet check`: configs in sync, then Prettier, cspell, markdownlint and the repo's own
//! check tasks one after another, then the house rules.

use crate::config::Repo;
use crate::contract::{Finding, RepoCheck, Severity};
use crate::rules::Rules;
use crate::sync;
use crate::tools::Tools;

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Let Prettier rewrite files. Off, it only checks, as CI does.
    pub write: bool,
    /// Run the Node tools and the repo's tasks. Off, only the config and house rules.
    pub tools: bool,
}

pub fn check(repo: &Repo, tools: &Tools, options: Options) -> RepoCheck {
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
        .collect();
    let mut runs = vec![];
    let mut formatted = vec![];
    if options.tools {
        let mut outcomes = vec![
            tools.prettier(&repo.root, options.write),
            tools.cspell(&repo.root),
            tools.markdownlint(&repo.root),
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
