//! A snapshot of the environment. Everything below `cli` reads variables from here, never
//! from the process, so tests can hand each run its own `HOME`, dirs and clock.

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct Env {
    vars: BTreeMap<String, String>,
}

impl Env {
    pub fn from_process() -> Env {
        Env::from_vars(
            std::env::vars_os()
                .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))),
        )
    }

    pub fn from_vars<K: Into<String>, V: Into<String>>(
        vars: impl IntoIterator<Item = (K, V)>,
    ) -> Env {
        Env {
            vars: vars
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }

    /// The value, when set and not empty.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars
            .get(key)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(key.into(), value.into());
    }

    pub fn remove(&mut self, key: &str) {
        self.vars.remove(key);
    }

    pub fn vars(&self) -> &BTreeMap<String, String> {
        &self.vars
    }

    pub fn home(&self) -> PathBuf {
        self.get("HOME")
            .map(PathBuf::from)
            .or_else(std::env::home_dir)
            .unwrap_or_else(|| PathBuf::from("/"))
    }
}
