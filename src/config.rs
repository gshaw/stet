//! stet's two config files.
//!
//! - `~/.config/stet/config.toml` (`STET_CONFIG_DIR`): which repos stet looks after and
//!   where `stet publish` writes. Optional; only `--all`, `inbox`, `doctor` and `publish`
//!   need it.
//! - `.stet.toml` at a repo's root: what that repo does differently from the shared setup.
//!   Its presence is what makes a folder a repo stet knows how to check.

use std::path::{Path, PathBuf};

use miette::{IntoDiagnostic, Result, WrapErr, miette};
use serde::Deserialize;

use crate::env::Env;

pub const REPO_FILE: &str = ".stet.toml";

/// The tools a `.stet.toml` ignore entry can apply to. `rules` is stet's own house rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Prettier,
    Markdownlint,
    Cspell,
    Rules,
}

pub const ALL_TOOLS: [Tool; 4] = [
    Tool::Prettier,
    Tool::Markdownlint,
    Tool::Cspell,
    Tool::Rules,
];

/// `~/.config/stet/config.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalFile {
    #[serde(default)]
    repos: Vec<String>,
    publish: Option<PublishFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishFile {
    site: String,
    #[serde(default = "default_articles")]
    articles: String,
}

fn default_articles() -> String {
    "content/articles".into()
}

#[derive(Debug, Clone)]
pub struct Global {
    pub home: PathBuf,
    pub config_dir: PathBuf,
    /// False when there's no config file; everything still works on a named repo.
    pub found: bool,
    pub repos: Vec<PathBuf>,
    pub publish: Option<Publish>,
}

#[derive(Debug, Clone)]
pub struct Publish {
    /// The site's repo, like ~/Developer/gshaw.ca.
    pub site: PathBuf,
    /// Where articles go inside it.
    pub articles: PathBuf,
}

impl Global {
    pub fn load(env: &Env) -> Result<Global> {
        let home = env.home();
        let config_dir = match env.get("STET_CONFIG_DIR") {
            Some(dir) => expand(dir, &home),
            None => home.join(".config/stet"),
        };
        let path = config_dir.join("config.toml");
        let (file, found) = match std::fs::read_to_string(&path) {
            Ok(text) => (
                toml::from_str::<GlobalFile>(&text)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("can't read {}", path.display()))?,
                true,
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (GlobalFile::default(), false),
            Err(e) => {
                return Err(e)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("can't read {}", path.display()));
            }
        };
        let repos = file.repos.iter().map(|r| expand(r, &home)).collect();
        let publish = file.publish.map(|p| {
            let site = expand(&p.site, &home);
            Publish {
                articles: site.join(p.articles),
                site,
            }
        });
        Ok(Global {
            home,
            config_dir,
            found,
            repos,
            publish,
        })
    }

    pub fn file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// The path with the home folder written as `~`, for people to read.
    pub fn abbreviate(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
            Ok(rest) if self.home != Path::new("/") => format!("~/{}", rest.display()),
            _ => path.display().to_string(),
        }
    }
}

/// Expands a leading `~`.
pub fn expand(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        home.to_path_buf()
    } else if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(path)
    }
}

/// `.stet.toml`: everything a repo does differently. Every key is optional.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoConfig {
    /// A knowledge repo (`notes`, the default) or a code repo (`code`). A code repo keeps
    /// its own `mise.toml` and CI, and gets only the hook and the Markdown checks.
    #[serde(default)]
    pub kind: Kind,
    /// Paths the tools skip, with the reason. Shared ignores (Inbox/, node_modules) come
    /// on top.
    #[serde(default)]
    pub ignore: Vec<Ignore>,
    #[serde(default)]
    pub frontmatter: Frontmatter,
    #[serde(default)]
    pub cspell: Cspell,
    #[serde(default)]
    pub mise: Mise,
    /// Write `.github/workflows/static-analysis.yml`. Default: yes for notes, no for code.
    ci: Option<bool>,
    /// Whether `stet publish` may copy notes from this repo to a public site.
    #[serde(default = "yes")]
    pub publish: bool,
    /// gitignore-style paths `stet publish` refuses even when `publish` is true.
    #[serde(default)]
    pub never_publish: Vec<String>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Notes,
    Code,
}

impl RepoConfig {
    /// Whether stet writes the CI workflow. A code repo runs `stet check` in its own.
    pub fn ci(&self) -> bool {
        self.ci.unwrap_or(self.kind == Kind::Notes)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ignore {
    /// gitignore-style patterns from the repo root: `Writing/pieces/`, `Journal/Diary/2*.md`.
    pub paths: Vec<String>,
    /// Which tools skip them. Absent means all four.
    pub tools: Option<Vec<Tool>>,
    /// Why. Written as a comment above the pattern in every generated file.
    pub reason: String,
}

impl Ignore {
    pub fn applies(&self, tool: Tool) -> bool {
        self.tools.as_ref().is_none_or(|t| t.contains(&tool))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frontmatter {
    /// gitignore-style patterns where YAML frontmatter is allowed, on top of the shared
    /// `AgentSkills/` and `.claude/`.
    #[serde(default)]
    pub allow: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cspell {
    /// Regular expressions cspell skips, in its `/…/flags` form.
    #[serde(default)]
    pub ignore_regexps: Vec<Pattern>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pattern {
    pub pattern: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mise {
    /// The repo's own mise tasks that `stet check` and CI run too, like an index check.
    #[serde(default)]
    pub check_tasks: Vec<String>,
}

/// A repo stet can check: its root and its `.stet.toml`.
#[derive(Debug, Clone)]
pub struct Repo {
    pub root: PathBuf,
    pub config: RepoConfig,
}

impl Repo {
    pub fn load(root: &Path) -> Result<Repo> {
        let path = root.join(REPO_FILE);
        let text = std::fs::read_to_string(&path)
            .into_diagnostic()
            .wrap_err_with(|| format!("can't read {}", path.display()))?;
        let config: RepoConfig = toml::from_str(&text)
            .into_diagnostic()
            .wrap_err_with(|| format!("can't read {}", path.display()))?;
        Ok(Repo {
            root: root.to_path_buf(),
            config,
        })
    }

    /// The repo holding `start`: the nearest folder at or above it with a `.stet.toml`.
    pub fn find(start: &Path) -> Result<Repo> {
        let start = std::path::absolute(start).into_diagnostic()?;
        let mut dir = if start.is_file() {
            start
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or(start.clone())
        } else {
            start.clone()
        };
        loop {
            if dir.join(REPO_FILE).is_file() {
                return Repo::load(&dir);
            }
            if !dir.pop() {
                return Err(miette!(
                    "no {REPO_FILE} in {} or any folder above it",
                    start.display()
                ));
            }
        }
    }

    pub fn name(&self) -> String {
        self.root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.root.display().to_string())
    }
}
