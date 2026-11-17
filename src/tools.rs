//! Runs Prettier, cspell and markdownlint-cli2 in a repo and turns their output into
//! findings. They run through `mise exec` at the versions in `generate.rs`, whatever the
//! repo pins, so mise installs each version once and every repo shares it. They run one
//! after another: Prettier writes files the other two read. Their configs are written to
//! stet's cache (`STET_CACHE_DIR`, else `$XDG_CACHE_HOME/stet` or `~/.cache/stet`) on every
//! run, never into the repo.
//!
//! `STET_PRETTIER`, `STET_CSPELL`, `STET_MARKDOWNLINT` and `STET_MISE` name a program to
//! run instead, so tests use stand-ins and never need Node.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;

use crate::config::Repo;
use crate::contract::{Finding, Severity, ToolRun};
use crate::env::Env;
use crate::generate::{self, CSPELL, MARKDOWNLINT, NODE, PRETTIER};

pub struct Tools {
    env: Env,
    mise: Option<PathBuf>,
    cache: PathBuf,
}

/// What one tool run produced.
pub struct Outcome {
    pub run: ToolRun,
    pub findings: Vec<Finding>,
    /// Files Prettier rewrote.
    pub formatted: Vec<String>,
}

struct Output {
    code: i32,
    text: String,
}

impl Tools {
    pub fn new(env: &Env) -> Tools {
        let mise = match env.get("STET_MISE") {
            Some(path) => Some(PathBuf::from(path)),
            None => find_mise(env),
        };
        let cache = match (env.get("STET_CACHE_DIR"), env.get("XDG_CACHE_HOME")) {
            (Some(dir), _) => PathBuf::from(dir),
            (None, Some(xdg)) => Path::new(xdg).join("stet"),
            (None, None) => env.home().join(".cache/stet"),
        };
        Tools {
            env: env.clone(),
            mise,
            cache,
        }
    }

    /// Writes one of a repo's tool configs to the cache and returns its path. Each repo has
    /// its own folder, named for it and a hash of its path. The write goes through a rename,
    /// so a hook and a full check running at once never read half a file.
    fn config_file(&self, repo: &Repo, name: &str, text: &str) -> Result<PathBuf, String> {
        let dir = self.cache.join("repos").join(format!(
            "{}-{:08x}",
            repo.name(),
            fnv(repo.root.display().to_string().as_bytes())
        ));
        let path = dir.join(name);
        let fail = |e: std::io::Error| format!("can't write {}: {e}", path.display());
        std::fs::create_dir_all(&dir).map_err(fail)?;
        let tmp = dir.join(format!(".{name}.{}", std::process::id()));
        std::fs::write(&tmp, text).map_err(fail)?;
        std::fs::rename(&tmp, &path).map_err(fail)?;
        Ok(path)
    }

    pub fn mise(&self) -> Option<&Path> {
        self.mise.as_deref()
    }

    /// The command for a Node tool: the override if set, else
    /// `mise exec node@… npm:<package>@… -- <name>` at stet's versions.
    fn command(&self, name: &str, override_key: &str) -> Option<Command> {
        let mut command = match self.env.get(override_key) {
            Some(program) => Command::new(program),
            None => {
                let mut c = Command::new(self.mise.as_ref()?);
                c.arg("exec").arg(format!("node@{NODE}"));
                if let Some(version) = version_of(name) {
                    c.arg(format!("npm:{name}@{version}"));
                }
                c.args(["--", name]);
                c
            }
        };
        command.env_clear().envs(self.env.vars());
        Some(command)
    }

    fn run<S: AsRef<std::ffi::OsStr>>(
        &self,
        name: &str,
        key: &str,
        root: &Path,
        args: &[S],
    ) -> Result<Output, String> {
        let mut command = self
            .command(name, key)
            .ok_or_else(|| format!("can't run {name}: mise isn't installed"))?;
        let output = command
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|e| format!("can't run {name}: {e}"))?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(Output {
            code: output.status.code().unwrap_or(-1),
            text,
        })
    }

    /// The tool's `--version`, to show it can run.
    pub fn version(&self, tool: &str, root: &Path) -> Result<String, String> {
        let (name, key) = match tool {
            "prettier" => ("prettier", "STET_PRETTIER"),
            "cspell" => ("cspell", "STET_CSPELL"),
            "markdownlint-cli2" => ("markdownlint-cli2", "STET_MARKDOWNLINT"),
            _ => ("node", "STET_NODE"),
        };
        let out = self.run(name, key, root, &["--version"])?;
        let first = out.text.lines().next().unwrap_or("").trim();
        let first = first
            .strip_prefix(&format!("{name} "))
            .unwrap_or(first)
            .to_string();
        if out.code == 0 {
            Ok(first)
        } else {
            Err(format!("{name} --version failed: {first}"))
        }
    }

    /// Prettier on every Markdown file: rewrites them, or only checks when `write` is false.
    pub fn prettier(&self, repo: &Repo, write: bool) -> Outcome {
        let mut args: Vec<String> = if write {
            vec!["--write".into(), "--list-different".into()]
        } else {
            vec!["--check".into()]
        };
        args.extend(generate::prettier_args(&repo.config));
        let mut outcome = Outcome::new("prettier");
        let out = match self.run("prettier", "STET_PRETTIER", &repo.root, &args) {
            Ok(out) => out,
            Err(e) => return outcome.failed(e),
        };
        for line in out
            .text
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.is_empty())
        {
            if let Some(rest) = line.strip_prefix("[warn] ") {
                if rest.starts_with("Code style issues") {
                    continue;
                }
                outcome.findings.push(finding(
                    Some(rest.into()),
                    None,
                    None,
                    "prettier",
                    "format",
                    "not formatted; `mise run check` formats it".into(),
                ));
            } else if let Some(rest) = line.strip_prefix("[error] ") {
                let (file, message) = rest.split_once(": ").unwrap_or(("", rest));
                outcome.findings.push(finding(
                    (!file.is_empty()).then(|| file.into()),
                    None,
                    None,
                    "prettier",
                    "error",
                    message.into(),
                ));
            } else if write && !line.starts_with("Checking formatting") {
                outcome.formatted.push(line.into());
            }
        }
        if out.code != 0 && outcome.findings.is_empty() {
            outcome
                .findings
                .push(tool_failed("prettier", out.code, &out.text));
        }
        outcome.finish()
    }

    pub fn cspell(&self, repo: &Repo) -> Outcome {
        static ISSUE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(.+?):(\d+):(\d+) - (.+?)(?: fix: \(.*\))?$").unwrap());
        static COUNT: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"Files checked: (\d+)").unwrap());
        let mut outcome = Outcome::new("cspell");
        let text = generate::cspell_config(&repo.config, &repo.root);
        let config = match self.config_file(repo, "cspell.yaml", &text) {
            Ok(path) => path,
            Err(e) => return outcome.failed(e),
        };
        let args = [
            "lint".as_ref(),
            "--no-progress".as_ref(),
            "--no-color".as_ref(),
            "-c".as_ref(),
            config.as_os_str(),
            ".".as_ref(),
        ];
        let out = match self.run::<&std::ffi::OsStr>("cspell", "STET_CSPELL", &repo.root, &args) {
            Ok(out) => out,
            Err(e) => return outcome.failed(e),
        };
        for line in out.text.lines() {
            if let Some(c) = ISSUE.captures(line) {
                outcome.findings.push(finding(
                    Some(c[1].into()),
                    c[2].parse().ok(),
                    c[3].parse().ok(),
                    "cspell",
                    "unknown-word",
                    format!("{}; a real word goes in .cspell.yaml", &c[4]),
                ));
            } else if let Some(c) = COUNT.captures(line) {
                outcome.run.files = c[1].parse().ok();
            }
        }
        if out.code != 0 && outcome.findings.is_empty() {
            outcome
                .findings
                .push(tool_failed("cspell", out.code, &out.text));
        }
        outcome.finish()
    }

    pub fn markdownlint(&self, repo: &Repo) -> Outcome {
        static ISSUE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(.+?):(\d+)(?::(\d+))? (?:(?:error|warning) )?(MD\d+\S*) (.*)$").unwrap()
        });
        static COUNT: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^Linting: (\d+) file").unwrap());
        let mut outcome = Outcome::new("markdownlint");
        let config = match self.config_file(
            repo,
            "markdownlint-cli2.yaml",
            &generate::markdownlint_config(),
        ) {
            Ok(path) => path,
            Err(e) => return outcome.failed(e),
        };
        let mut args = vec!["--config".to_string(), config.display().to_string()];
        args.extend(generate::markdownlint_globs(&repo.config));
        let out = match self.run("markdownlint-cli2", "STET_MARKDOWNLINT", &repo.root, &args) {
            Ok(out) => out,
            Err(e) => return outcome.failed(e),
        };
        for line in out.text.lines() {
            if let Some(c) = ISSUE.captures(line) {
                outcome.findings.push(finding(
                    Some(c[1].into()),
                    c[2].parse().ok(),
                    c.get(3).and_then(|m| m.as_str().parse().ok()),
                    "markdownlint",
                    &c[4],
                    c[5].into(),
                ));
            } else if let Some(c) = COUNT.captures(line) {
                outcome.run.files = c[1].parse().ok();
            }
        }
        if out.code != 0 && outcome.findings.is_empty() {
            outcome
                .findings
                .push(tool_failed("markdownlint", out.code, &out.text));
        }
        outcome.finish()
    }

    /// One of the repo's own mise tasks, like Arniston's `index-check`.
    pub fn task(&self, root: &Path, task: &str) -> Outcome {
        let mut outcome = Outcome::new(&format!("mise:{task}"));
        let Some(mise) = &self.mise else {
            return outcome.failed(format!("can't run mise task {task}: mise isn't installed"));
        };
        let result = Command::new(mise)
            .args(["run", "--quiet", task])
            .env_clear()
            .envs(self.env.vars())
            .current_dir(root)
            .output();
        match result {
            Ok(out) if out.status.success() => {}
            Ok(out) => {
                let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
                text.push_str(&String::from_utf8_lossy(&out.stderr));
                let mut f = tool_failed("mise", out.status.code().unwrap_or(-1), &text);
                f.rule = task.into();
                outcome.findings.push(f);
            }
            Err(e) => return outcome.failed(format!("can't run mise task {task}: {e}")),
        }
        outcome.finish()
    }
}

impl Outcome {
    fn new(name: &str) -> Outcome {
        Outcome {
            run: ToolRun {
                name: name.into(),
                ok: true,
                files: None,
            },
            findings: vec![],
            formatted: vec![],
        }
    }

    fn failed(mut self, message: String) -> Outcome {
        let tool = self.run.name.clone();
        self.findings.push(Finding {
            file: None,
            line: None,
            column: None,
            tool,
            rule: "run".into(),
            severity: Severity::Error,
            message,
        });
        self.finish()
    }

    fn finish(mut self) -> Outcome {
        self.run.ok = self.findings.is_empty();
        self
    }
}

fn finding(
    file: Option<String>,
    line: Option<usize>,
    column: Option<usize>,
    tool: &str,
    rule: &str,
    message: String,
) -> Finding {
    Finding {
        file,
        line,
        column,
        tool: tool.into(),
        rule: rule.into(),
        severity: Severity::Error,
        message,
    }
}

/// A failure the parser didn't recognise: the exit code and the output's last lines.
fn tool_failed(tool: &str, code: i32, text: &str) -> Finding {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let tail = lines[lines.len().saturating_sub(5)..].join(" / ");
    finding(
        None,
        None,
        None,
        tool,
        "run",
        format!("exited {code}: {tail}"),
    )
}

/// The npm package version stet pins for a tool; `None` for Node itself.
fn version_of(name: &str) -> Option<&'static str> {
    match name {
        "prettier" => Some(PRETTIER),
        "cspell" => Some(CSPELL),
        "markdownlint-cli2" => Some(MARKDOWNLINT),
        _ => None,
    }
}

/// FNV-1a: a hash that stays the same across Rust versions, for cache folder names.
fn fnv(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |h, b| {
        (h ^ u32::from(*b)).wrapping_mul(0x0100_0193)
    })
}

/// mise on PATH, then the usual install places.
fn find_mise(env: &Env) -> Option<PathBuf> {
    let home = env.home();
    let mut dirs: Vec<PathBuf> = env
        .get("PATH")
        .map(|p| std::env::split_paths(p).collect())
        .unwrap_or_default();
    dirs.push(home.join(".local/bin"));
    dirs.push("/opt/homebrew/bin".into());
    dirs.push("/usr/local/bin".into());
    dirs.into_iter()
        .map(|d| d.join("mise"))
        .find(|p| p.is_file())
}
