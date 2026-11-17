//! `stet init <dir>`: a new knowledge repo, as Workshop's `Meta/README.md` bootstraps one.
//! It writes files and runs `git init`; creating the GitHub repo, pushing and the first
//! content are left to the person, and listed in `next`.

use std::path::Path;
use std::process::Command;

use miette::{IntoDiagnostic, Result, WrapErr, miette};

use crate::config::{REPO_FILE, Repo};
use crate::contract::{InitReport, SCHEMA};
use crate::env::Env;
use crate::sync;

const README: &str = include_str!("templates/README.md");
const AGENTS: &str = include_str!("templates/AGENTS.md");
const GITIGNORE: &str = include_str!("templates/gitignore");
const STET_TOML: &str = include_str!("templates/stet.toml");

/// What `dir` may already hold: an empty `git init` is fine, anything else isn't.
const ALLOWED: [&str; 2] = [".git", ".DS_Store"];

pub fn init(env: &Env, dir: &Path, name: Option<&str>) -> Result<InitReport> {
    let dir = std::path::absolute(dir).into_diagnostic()?;
    let name = match name {
        Some(n) => n.to_string(),
        None => dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or_else(|| miette!("can't name a repo at {}; pass --name", dir.display()))?,
    };
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let extra: Vec<String> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| !ALLOWED.contains(&n.as_str()))
            .collect();
        if !extra.is_empty() {
            return Err(miette!(
                "{} isn't empty ({}); stet init only starts new repos",
                dir.display(),
                extra.join(", ")
            ));
        }
    }
    std::fs::create_dir_all(&dir)
        .into_diagnostic()
        .wrap_err_with(|| format!("can't create {}", dir.display()))?;

    let mut created = vec![];
    if !dir.join(".git").exists() {
        git_init(env, &dir)?;
        created.push(".git/".to_string());
    }
    let files = [
        (REPO_FILE, STET_TOML.to_string()),
        (".gitignore", GITIGNORE.to_string()),
        ("README.md", README.replace("{name}", &name)),
        ("AGENTS.md", AGENTS.replace("{name}", &name)),
    ];
    for (path, text) in files {
        std::fs::write(dir.join(path), text).into_diagnostic()?;
        created.push(path.to_string());
    }
    std::os::unix::fs::symlink("AGENTS.md", dir.join("CLAUDE.md")).into_diagnostic()?;
    created.push("CLAUDE.md".into());
    std::fs::create_dir_all(dir.join("Inbox")).into_diagnostic()?;
    created.push("Inbox/".into());
    let repo = Repo::load(&dir)?;
    for file in sync::sync(&repo)?.files {
        created.push(file.path);
    }
    created.sort();

    let next = vec![
        "Fill in the ❌ lines in README.md and AGENTS.md.".to_string(),
        "Create the private repo on GitHub and add it as origin.".to_string(),
        "Add the repo to ~/.config/stet/config.toml.".to_string(),
        "`mise trust && mise install`, then `mise run check` until clean.".to_string(),
        "`hk install --mise` to turn on the pre-commit check.".to_string(),
        "Import the first content as its first domain, commit to main and push.".to_string(),
    ];
    Ok(InitReport {
        schema: SCHEMA,
        path: dir.display().to_string(),
        name,
        created,
        next,
    })
}

fn git_init(env: &Env, dir: &Path) -> Result<()> {
    let git = env.get("STET_GIT").unwrap_or("git");
    let output = Command::new(git)
        .args(["init", "-q", "-b", "main"])
        .current_dir(dir)
        .env_clear()
        .envs(env.vars())
        .output()
        .into_diagnostic()
        .wrap_err("can't run git")?;
    if !output.status.success() {
        return Err(miette!(
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}
