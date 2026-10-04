//! `stet doctor`: everything `stet check` and the other commands need, checked once.

use crate::config::{Global, Repo};
use crate::contract::{CheckStatus, DoctorCheck, DoctorReport, SCHEMA};
use crate::sync;
use crate::tools::Tools;

pub fn doctor(global: &Global, tools: &Tools) -> DoctorReport {
    let mut checks: Vec<DoctorCheck> = vec![];
    let mut add = |id: &str, status: CheckStatus, message: String| {
        checks.push(DoctorCheck {
            id: id.into(),
            status,
            message,
        })
    };
    let file = global.abbreviate(&global.file());
    if global.found {
        let n = global.repos.len();
        add(
            "config",
            CheckStatus::Ok,
            format!("{file}: {n} repo{}", if n == 1 { "" } else { "s" }),
        );
    } else {
        add(
            "config",
            CheckStatus::Warning,
            format!("no {file}; --all, inbox and publish need one (stet help config)"),
        );
    }

    match tools.mise() {
        Some(path) => add("mise", CheckStatus::Ok, path.display().to_string()),
        None => add(
            "mise",
            CheckStatus::Error,
            "mise isn't on PATH, in ~/.local/bin or in Homebrew; stet runs the tools through it"
                .into(),
        ),
    }

    // The tools run in a repo, with its pinned versions; any repo will do to see they run.
    let probe = global
        .repos
        .iter()
        .find(|r| r.is_dir())
        .cloned()
        .unwrap_or_else(|| global.home.clone());
    for tool in ["node", "prettier", "cspell", "markdownlint-cli2"] {
        match tools.version(tool, &probe) {
            Ok(version) => add(tool, CheckStatus::Ok, version),
            Err(e) => add(tool, CheckStatus::Error, e),
        }
    }

    for path in &global.repos {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let id = format!("repo:{name}");
        let shown = global.abbreviate(path);
        if !path.is_dir() {
            add(&id, CheckStatus::Error, format!("no folder at {shown}"));
            continue;
        }
        match Repo::load(path) {
            Ok(repo) => {
                let drifted = sync::drifted(&repo);
                if drifted.is_empty() {
                    add(&id, CheckStatus::Ok, format!("{shown}: configs in sync"));
                } else {
                    add(
                        &id,
                        CheckStatus::Error,
                        format!(
                            "{shown}: {} {} from .stet.toml; run `stet sync`",
                            drifted.join(", "),
                            if drifted.len() == 1 {
                                "differs"
                            } else {
                                "differ"
                            }
                        ),
                    );
                }
            }
            Err(e) => add(&id, CheckStatus::Error, crate::chain(&e)),
        }
    }

    match &global.publish {
        None => add(
            "publish",
            CheckStatus::Warning,
            format!("no [publish] in {file}; stet publish needs a site"),
        ),
        Some(p) if !p.site.is_dir() => add(
            "publish",
            CheckStatus::Error,
            format!("no site at {}", global.abbreviate(&p.site)),
        ),
        Some(p) if !p.articles.is_dir() => add(
            "publish",
            CheckStatus::Error,
            format!("no articles folder at {}", global.abbreviate(&p.articles)),
        ),
        Some(p) => add(
            "publish",
            CheckStatus::Ok,
            format!("articles go to {}", global.abbreviate(&p.articles)),
        ),
    }

    let ok = checks.iter().all(|c| c.status != CheckStatus::Error);
    DoctorReport {
        schema: SCHEMA,
        ok,
        checks,
    }
}
