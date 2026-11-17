//! `stet sync` writes the files stet keeps in a repo and removes the tool configs an older
//! stet generated there; `stet check` reports both.

use miette::{IntoDiagnostic, Result, WrapErr};

use crate::config::{Kind, Repo};
use crate::contract::{RepoSync, SyncAction, SyncFile};
use crate::generate::{self, Generated};

pub fn expected(repo: &Repo) -> Vec<Generated> {
    generate::files(&repo.config, |path| {
        std::fs::read_to_string(repo.root.join(path)).ok()
    })
}

/// The generated files whose copy in the repo is missing or different.
pub fn drifted(repo: &Repo) -> Vec<&'static str> {
    expected(repo)
        .into_iter()
        .filter(|g| {
            std::fs::read_to_string(repo.root.join(g.path))
                .ok()
                .as_ref()
                != Some(&g.text)
        })
        .map(|g| g.path)
        .collect()
}

/// Tool configs an older stet generated into the repo. They're written to the cache now.
pub fn leftover(repo: &Repo) -> Vec<&'static str> {
    generate::LEGACY
        .into_iter()
        .filter(|path| {
            std::fs::read_to_string(repo.root.join(path)).is_ok_and(|t| generate::is_generated(&t))
        })
        .collect()
}

/// cspell config files under another name. stet imports only `.cspell.yaml`, so their
/// words would be skipped without a word.
pub const OTHER_CSPELL: [&str; 6] = [
    ".cspell.yml",
    ".cspell.json",
    "cspell.json",
    "cspell.yaml",
    "cspell.config.yaml",
    "cspell.config.yml",
];

pub fn other_cspell(repo: &Repo) -> Vec<&'static str> {
    OTHER_CSPELL
        .into_iter()
        .filter(|p| repo.root.join(p).is_file())
        .collect()
}

/// A code repo pins stet and hk in its own mise config, and turns off mise's version
/// service, which doesn't list stet. What's missing or behind, as (file, message).
pub fn pins(repo: &Repo) -> Vec<(String, String)> {
    if repo.config.kind != Kind::Code {
        return vec![];
    }
    let mut found: Vec<(&str, Option<String>, Option<String>)> = vec![];
    let mut host_off = false;
    for file in ["mise.toml", ".mise.toml"] {
        let Ok(text) = std::fs::read_to_string(repo.root.join(file)) else {
            continue;
        };
        let Ok(table) = toml::from_str::<toml::Table>(&text) else {
            continue;
        };
        host_off |= table
            .get("settings")
            .and_then(|s| s.get("use_versions_host"))
            .and_then(|v| v.as_bool())
            == Some(false);
        let tools = table.get("tools").and_then(|t| t.as_table());
        let version = |name: &str| {
            let v = tools?.get(name)?;
            v.as_str()
                .or_else(|| v.get("version").and_then(|v| v.as_str()))
                .map(String::from)
        };
        found.push((file, version(generate::SOURCE), version("hk")));
    }
    let file = found.first().map_or("mise.toml", |f| f.0).to_string();
    let stet = found.iter().find_map(|f| f.1.clone());
    let hk = found.iter().find_map(|f| f.2.clone());
    let mut out = vec![];
    for (name, want, have) in [
        (generate::SOURCE, generate::STET, stet),
        ("hk", generate::HK, hk),
    ] {
        if have.as_deref() != Some(want) {
            let now = match have {
                Some(v) => format!("pins {v}"),
                None => "doesn't pin it".into(),
            };
            out.push((
                file.clone(),
                format!("{now}; pin `\"{name}\" = \"{want}\"` under [tools]"),
            ));
        }
    }
    if !host_off {
        out.push((
            file,
            "set `use_versions_host = false` under [settings]: mise's version service \
             doesn't list stet, so the pin can't install"
                .into(),
        ));
    }
    out
}

pub fn sync(repo: &Repo) -> Result<RepoSync> {
    let mut files = vec![];
    for generated in expected(repo) {
        let path = repo.root.join(generated.path);
        let same = std::fs::read_to_string(&path).ok().as_ref() == Some(&generated.text);
        if !same {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).into_diagnostic()?;
            }
            std::fs::write(&path, &generated.text)
                .into_diagnostic()
                .wrap_err_with(|| format!("can't write {}", path.display()))?;
        }
        files.push(SyncFile {
            path: generated.path.into(),
            action: if same {
                SyncAction::Unchanged
            } else {
                SyncAction::Written
            },
        });
    }
    for path in leftover(repo) {
        let full = repo.root.join(path);
        std::fs::remove_file(&full)
            .into_diagnostic()
            .wrap_err_with(|| format!("can't remove {}", full.display()))?;
        files.push(SyncFile {
            path: path.into(),
            action: SyncAction::Removed,
        });
    }
    Ok(RepoSync {
        name: repo.name(),
        path: repo.root.display().to_string(),
        files,
    })
}
