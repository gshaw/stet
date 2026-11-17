//! `stet sync` writes the files stet keeps in a repo and removes the tool configs an older
//! stet generated there; `stet check` reports both.

use miette::{IntoDiagnostic, Result, WrapErr};

use crate::config::Repo;
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
