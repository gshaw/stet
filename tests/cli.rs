//! Runs the built `stet` against a copy of tests/fixtures, with fake Node tools and a fake
//! mise. Nothing here touches the real knowledge repos, ~/.config or gshaw.ca.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const STET: &str = env!("CARGO_BIN_EXE_stet");

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

struct Output {
    status: i32,
    stdout: String,
    stderr: String,
}

impl Output {
    fn json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("{e}: {}\n{}", self.stdout, self.stderr))
    }
}

/// A home folder of its own, with the fixture repo at `home/Repo`.
struct Home {
    _dir: tempfile::TempDir,
    root: PathBuf,
    env: Vec<(String, String)>,
}

impl Home {
    fn new() -> Home {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        copy_dir(&fixtures().join("repo"), &root.join("home/Repo"));
        std::fs::create_dir_all(root.join("fake")).unwrap();
        let bin = fixtures().join("bin");
        let path = |p: &str| root.join(p).display().to_string();
        let tool = |n: &str| bin.join(n).display().to_string();
        let env = vec![
            ("HOME", path("home")),
            ("PATH", "/usr/bin:/bin".into()),
            ("STET_CONFIG_DIR", path("home/.config/stet")),
            ("STET_TODAY", "2026-10-04".into()),
            ("STET_PRETTIER", tool("prettier")),
            ("STET_CSPELL", tool("cspell")),
            ("STET_MARKDOWNLINT", tool("markdownlint-cli2")),
            ("STET_MISE", tool("mise")),
            ("STET_NODE", tool("node")),
            ("FAKE_LOG", path("fake/calls.log")),
            ("FAKE_DIR", path("fake")),
        ];
        Home {
            _dir: dir,
            root,
            env: env.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        }
    }

    fn repo(&self) -> PathBuf {
        self.root.join("home/Repo")
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// Canned output for a fake tool.
    fn fake(&self, tool: &str, out: &str, code: i32) {
        self.write(&format!("fake/{tool}.out"), out);
        self.write(&format!("fake/{tool}.code"), &code.to_string());
    }

    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.root.join("fake/calls.log"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }

    fn stet_in(&self, cwd: &Path, args: &[&str]) -> Output {
        let output = Command::new(STET)
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .envs(self.env.iter().map(|(k, v)| (k, v)))
            .output()
            .unwrap();
        Output {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn stet(&self, args: &[&str]) -> Output {
        self.stet_in(&self.repo(), args)
    }

    fn snapshot(&self, name: &str, value: &Value) {
        let root = regex_escape(&self.root.display().to_string());
        let repo = regex_escape(env!("CARGO_MANIFEST_DIR"));
        let filters = vec![(root.as_str(), "[ROOT]"), (repo.as_str(), "[STET]")];
        insta::with_settings!({ filters => filters }, {
            insta::assert_json_snapshot!(name, value);
        });
    }
}

fn regex_escape(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if "\\.+*?()|[]{}^$".contains(c) {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect()
}

fn rules(report: &Value) -> Vec<String> {
    report["repos"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            format!(
                "{} {}:{} {}",
                f["rule"].as_str().unwrap(),
                f["file"].as_str().unwrap_or("-"),
                f["line"]
                    .as_u64()
                    .map(|l| l.to_string())
                    .unwrap_or_default(),
                f["severity"].as_str().unwrap()
            )
        })
        .collect()
}

#[test]
fn sync_writes_the_configs_once() {
    let home = Home::new();
    let first = home.stet(&["sync", "--json"]);
    assert_eq!(first.status, 0, "{}", first.stderr);
    home.snapshot("sync", &first.json());
    let again = home.stet(&["sync", "--json"]).json();
    for file in again["repos"][0]["files"].as_array().unwrap() {
        assert_eq!(file["action"], "unchanged", "{file}");
    }
    let mise = std::fs::read_to_string(home.repo().join(".mise.toml")).unwrap();
    assert!(mise.contains(r#"depends = ["ci-format-md", "spell", "markdownlint", "extra-check"]"#));
    let ignore = std::fs::read_to_string(home.repo().join(".prettierignore")).unwrap();
    assert!(ignore.contains("# Kept exactly as written.\nVerbatim/\n"));
}

#[test]
fn sync_keeps_the_repo_words_and_tasks() {
    let home = Home::new();
    home.write(
        "home/Repo/.cspell.yaml",
        "version: \"0.2\"\n# Our words\nwords:\n  - Gerry\n",
    );
    home.stet(&["sync"]);
    let mise = std::fs::read_to_string(home.repo().join(".mise.toml")).unwrap();
    std::fs::write(
        home.repo().join(".mise.toml"),
        format!("{mise}\n[tasks.extra-check]\nrun = \"true\"\n"),
    )
    .unwrap();
    home.stet(&["sync"]);
    let cspell = std::fs::read_to_string(home.repo().join(".cspell.yaml")).unwrap();
    assert!(cspell.starts_with("# Generated by stet"));
    assert!(cspell.ends_with("\n# Our words\nwords:\n  - Gerry\n"));
    let mise = std::fs::read_to_string(home.repo().join(".mise.toml")).unwrap();
    assert!(mise.ends_with("[tasks.extra-check]\nrun = \"true\"\n"));
}

#[test]
fn check_applies_the_house_rules() {
    let home = Home::new();
    home.write(
        "home/Repo/Inbox/raw.md",
        "---\nx: 1\n---\n[dead](nowhere.md)\n",
    );
    home.stet(&["sync"]);
    let out = home.stet(&["check", "--rules-only", "--json"]);
    assert_eq!(out.status, 1);
    let report = out.json();
    assert_eq!(
        rules(&report),
        [
            "superseded-date Notes/a.md:3 error",
            "frontmatter Notes/front.md:1 error",
            "link README.md:4 error",
            "link README.md:4 error",
            "anchor README.md:5 error",
            "undated-now Notes/a.md:9 warning",
        ]
    );
    home.snapshot("check-rules", &report);
    assert!(home.calls().is_empty(), "{:?}", home.calls());
}

#[test]
fn check_runs_the_tools_in_order_and_parses_them() {
    let home = Home::new();
    home.stet(&["sync"]);
    home.fake("prettier", "Notes/a.md\n", 0);
    home.fake(
        "cspell",
        "Notes/a.md:7:4 - Unknown word (curently) fix: (currently)\n\
         CSpell: Files checked: 6, Issues found: 1 in 1 file.\n",
        1,
    );
    home.fake(
        "markdownlint-cli2",
        "markdownlint-cli2 v0.22.1 (markdownlint v0.40.0)\nLinting: 5 file(s)\n\
         Notes/a.md:3 error MD022/blanks-around-headings Headings should be surrounded by blank lines [Expected: 1; Actual: 0; Below]\n",
        1,
    );
    let out = home.stet(&["check", "--json"]);
    assert_eq!(out.status, 1);
    assert_eq!(
        home.calls(),
        [
            "prettier --write --list-different **/*.md",
            "cspell lint --no-progress --no-color .",
            "markdownlint-cli2 **/*.md",
            "mise run --quiet extra-check",
        ]
    );
    let report = out.json();
    home.snapshot("check-tools", &report);
    assert_eq!(report["repos"][0]["formatted"][0], "Notes/a.md");
}

#[test]
fn check_only_never_writes() {
    let home = Home::new();
    home.stet(&["sync"]);
    home.fake(
        "prettier",
        "Checking formatting...\n[warn] Notes/a.md\n[warn] Code style issues found in the above file. Run Prettier with --write to fix.\n",
        1,
    );
    let out = home.stet(&["check", "--check-only", "--json"]);
    assert_eq!(home.calls()[0], "prettier --check **/*.md");
    let report = out.json();
    let prettier: Vec<_> = report["repos"][0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["tool"] == "prettier")
        .collect();
    assert_eq!(prettier.len(), 1);
    assert_eq!(prettier[0]["file"], "Notes/a.md");
}

#[test]
fn check_files_keeps_only_their_findings() {
    let home = Home::new();
    home.write("home/Repo/Notes/untracked.md", "[dead](nowhere.md)\n");
    home.stet(&["sync"]);
    let out = home.stet(&["check", "--rules-only", "--json", "--files", "Notes/a.md"]);
    assert_eq!(out.status, 1, "{}", out.stderr);
    assert_eq!(
        rules(&out.json()),
        [
            "superseded-date Notes/a.md:3 error",
            "undated-now Notes/a.md:9 warning"
        ]
    );
    let sub = home.repo().join("Notes");
    let out = home.stet_in(
        &sub,
        &["check", "--rules-only", "--json", "--files", "front.md"],
    );
    assert_eq!(rules(&out.json()), ["frontmatter Notes/front.md:1 error"]);
    let out = home.stet(&["check", "--rules-only", "--files", "Drafts/post.md"]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    let out = home.stet(&["check", "--rules-only", "--files", "/elsewhere/x.md"]);
    assert_eq!(out.status, 1);
    assert!(out.stderr.contains("isn't in"), "{}", out.stderr);
}

#[test]
fn check_fails_when_a_config_drifts() {
    let home = Home::new();
    home.stet(&["sync"]);
    home.write("home/Repo/.prettierrc.yaml", "proseWrap: always\n");
    let report = home.stet(&["check", "--rules-only", "--json"]).json();
    assert!(rules(&report).contains(&"config-sync .prettierrc.yaml: error".to_string()));
    home.stet(&["sync"]);
    let report = home.stet(&["check", "--rules-only", "--json"]).json();
    assert!(!rules(&report).iter().any(|r| r.starts_with("config-sync")));
}

#[test]
fn warnings_alone_pass() {
    let home = Home::new();
    for file in ["README.md", "Notes/a.md", "Notes/front.md"] {
        std::fs::remove_file(home.repo().join(file)).unwrap();
    }
    home.write("home/Repo/Notes/b.md", "We currently do it by hand.\n");
    home.stet(&["sync"]);
    let out = home.stet(&["check", "--rules-only"]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    assert!(
        out.stdout.contains("Repo: clean, 1 warning"),
        "{}",
        out.stdout
    );
}

#[test]
fn check_finds_the_repo_from_a_subfolder_and_by_path() {
    let home = Home::new();
    home.stet(&["sync"]);
    let sub = home.stet_in(
        &home.repo().join("Notes"),
        &["check", "--rules-only", "--json"],
    );
    assert_eq!(sub.json()["repos"][0]["name"], "Repo");
    let by_path = home.stet_in(
        &home.root,
        &["check", "--rules-only", "--json", "home/Repo"],
    );
    assert_eq!(by_path.json()["repos"][0]["name"], "Repo");
}

#[test]
fn check_all_reads_the_config() {
    let home = Home::new();
    home.stet(&["sync"]);
    home.write("home/.config/stet/config.toml", "repos = [\"~/Repo\"]\n");
    let out = home.stet_in(&home.root, &["check", "--all", "--rules-only", "--json"]);
    assert_eq!(out.json()["repos"].as_array().unwrap().len(), 1);
}

#[test]
fn outside_a_repo_is_an_error_document() {
    let home = Home::new();
    let out = home.stet_in(&home.root, &["check", "--json"]);
    assert_eq!(out.status, 1);
    home.snapshot("error", &out.json());
}

#[test]
fn help_topics_print() {
    let home = Home::new();
    let out = home.stet(&["help", "config"]);
    assert!(out.stdout.starts_with("# .stet.toml"));
    let out = home.stet(&["help", "rules"]);
    for rule in [
        "config-sync",
        "frontmatter",
        "link",
        "anchor",
        "superseded-date",
        "undated-now",
    ] {
        assert!(out.stdout.contains(&format!("**`{rule}`**")), "{rule}");
    }
    assert_eq!(home.stet(&["help", "nope"]).status, 1);
    assert_eq!(home.stet(&["nope"]).status, 64);
}

#[test]
fn doctor_reports_each_part() {
    let home = Home::new();
    home.stet(&["sync"]);
    for tool in ["node", "prettier", "cspell", "markdownlint-cli2"] {
        home.fake(tool, &format!("{tool} 1.0.0\n"), 0);
    }
    let out = home.stet(&["doctor", "--json"]);
    assert_eq!(out.status, 0, "{}", out.stdout);
    home.snapshot("doctor-no-config", &out.json());
    home.write(
        "home/.config/stet/config.toml",
        "repos = [\"~/Repo\", \"~/Gone\"]\n[publish]\nsite = \"~/site\"\n",
    );
    home.write("home/site/content/articles/_index.md", "");
    home.write("home/Repo/.prettierrc.yaml", "drifted\n");
    let report = home.stet(&["doctor", "--json"]).json();
    home.snapshot("doctor", &report);
    assert_eq!(report["ok"], false);
}

#[test]
fn inbox_lists_items_and_mail_rules() {
    let home = Home::new();
    for file in [
        "README.md",
        ".gitignore",
        ".DS_Store",
        "_processed/2026-10-01/old.pdf",
        "scan.pdf",
        "trip/a.eml",
        "trip/b.eml",
    ] {
        home.write(&format!("home/Repo/Inbox/{file}"), "12345");
    }
    home.write("home/.config/stet/config.toml", "repos = [\"~/Repo\"]\n");
    let report = home.stet_in(&home.root, &["inbox", "--json"]).json();
    let mut redacted = report.clone();
    for item in redacted["repos"][0]["items"].as_array_mut().unwrap() {
        item["modified"] = "[TIME]".into();
    }
    home.snapshot("inbox", &redacted);
    assert_eq!(report["waiting"], 2);
    assert_eq!(report["repos"][0]["items"][1]["bytes"], 10);
    let out = home.stet_in(&home.root, &["inbox", "--mail", "home/Repo"]);
    assert!(out.stdout.contains("Repo  2 waiting"), "{}", out.stdout);
    assert!(
        out.stdout.contains("  mail: Travel bookings and tickets."),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("  not here: health or money."),
        "{}",
        out.stdout
    );
}

#[test]
fn inbox_says_when_the_mail_section_is_missing() {
    let home = Home::new();
    home.write("home/Repo/AGENTS.md", "# Rules\n");
    let report = home
        .stet_in(&home.root, &["inbox", "--json", "home/Repo"])
        .json();
    assert_eq!(
        report["repos"][0]["problem"],
        "AGENTS.md has no \"Mail that belongs here\" section"
    );
    assert!(report["repos"][0].get("mail").is_none());
}

#[test]
fn init_starts_a_repo_that_passes_its_own_check() {
    let home = Home::new();
    let out = home.stet_in(&home.root, &["init", "--json", "home/Music"]);
    assert_eq!(out.status, 0, "{}", out.stderr);
    home.snapshot("init", &out.json());
    let repo = home.root.join("home/Music");
    assert_eq!(
        std::fs::read_link(repo.join("CLAUDE.md")).unwrap(),
        Path::new("AGENTS.md")
    );
    assert!(repo.join(".git").is_dir() && repo.join("Inbox").is_dir());
    let readme = std::fs::read_to_string(repo.join("README.md")).unwrap();
    assert!(readme.starts_with("# Music\n"));
    let check = home
        .stet_in(&repo, &["check", "--rules-only", "--json"])
        .json();
    assert_eq!(check["errors"], 0, "{check}");
    assert_eq!(check["warnings"], 0, "{check}");
    let inbox = home
        .stet_in(&home.root, &["inbox", "--json", "home/Music"])
        .json();
    assert!(inbox["repos"][0].get("problem").is_none(), "{inbox}");
}

#[test]
fn init_refuses_a_folder_with_files() {
    let home = Home::new();
    let out = home.stet_in(&home.root, &["init", "home/Repo"]);
    assert_eq!(out.status, 1);
    assert!(out.stderr.contains("isn't empty"), "{}", out.stderr);
    assert!(!home.repo().join("CLAUDE.md").exists());
}

fn publish_home() -> Home {
    let home = Home::new();
    home.write(
        "home/.config/stet/config.toml",
        "repos = [\"~/Repo\"]\n[publish]\nsite = \"~/site\"\n",
    );
    home.write(
        "home/site/content/articles/_index.md",
        "---\ntitle: Articles\n---\n",
    );
    home.write(
        "home/Repo/Notes/post.md",
        "# Backups that work\n\nSee [the plan](plan.md#steps), [Litestream](https://litestream.io)\n\
         and [Life's note](https://github.com/gshaw/Life/blob/main/x.md).\n\n\
         ![diagram](diagram.png)\n\n## Steps\n\nStop the old machine first.\n",
    );
    home
}

#[test]
fn publish_writes_a_draft_without_private_links() {
    let home = publish_home();
    let out = home.stet(&["publish", "--json", "Notes/post.md"]);
    assert_eq!(out.status, 0, "{}{}", out.stdout, out.stderr);
    home.snapshot("publish", &out.json());
    let page =
        std::fs::read_to_string(home.root.join("home/site/content/articles/post.md")).unwrap();
    assert_eq!(
        page,
        "---\ntitle: \"Backups that work\"\ndate: 2026-10-04\ndraft: true\n---\n\n\
         See the plan, [Litestream](https://litestream.io)\nand [Life's note](https://github.com/gshaw/Life/blob/main/x.md).\n\n\
         \n\n## Steps\n\nStop the old machine first.\n"
    );
    let again = home.stet(&["publish", "Notes/post.md"]);
    assert_eq!(again.status, 1);
    assert!(again.stderr.contains("--force"), "{}", again.stderr);
    let forced = home.stet(&["publish", "--force", "--slug", "Backups", "Notes/post.md"]);
    assert!(
        forced.stdout.ends_with("articles/backups.md\n"),
        "{}",
        forced.stdout
    );
}

#[test]
fn publish_refuses_secrets_flags_and_private_repos() {
    let home = publish_home();
    home.write(
        "home/Repo/Notes/keys.md",
        "# Keys\n\n⚠️ publish-review: door codes\n\ntoken: ghp_abcdefghijklmnopqrstuvwxyz0123456789\n",
    );
    let out = home.stet(&["publish", "--json", "Notes/keys.md"]);
    assert_eq!(out.status, 1);
    home.snapshot("publish-refused", &out.json());

    let mut toml = std::fs::read_to_string(home.repo().join(".stet.toml")).unwrap();
    toml.insert_str(0, "never_publish = [\"Notes/post.md\"]\n");
    std::fs::write(home.repo().join(".stet.toml"), &toml).unwrap();
    let out = home.stet(&["publish", "Notes/post.md"]);
    assert!(out.stderr.contains("never_publish"), "{}", out.stderr);

    std::fs::write(
        home.repo().join(".stet.toml"),
        format!("publish = false\n{toml}"),
    )
    .unwrap();
    let out = home.stet(&["publish", "Notes/post.md"]);
    assert_eq!(out.status, 1);
    assert!(
        out.stderr.contains("Repo notes are never published"),
        "{}",
        out.stderr
    );
    assert!(
        !home
            .root
            .join("home/site/content/articles/post.md")
            .exists()
    );
}
