//! The `stet` command: flags, dispatch and human output. The work happens in the library;
//! this file only turns it into text or JSON.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::env::{Bash, CompleteEnv, EnvCompleter, Fish, Zsh};
use miette::{IntoDiagnostic, Result, miette};
use serde::Serialize;

use crate::check::{self, Options};
use crate::config::{Global, Repo};
use crate::contract::{CheckReport, ErrorReport, SCHEMA, Severity, SyncAction, SyncReport};
use crate::env::Env;
use crate::help;
use crate::sync;
use crate::tools::Tools;

/// Keep Markdown knowledge repos in shape.
#[derive(Debug, Parser)]
#[command(
    name = "stet",
    version = crate::VERSION,
    disable_help_subcommand = true,
    after_help = "Repos and the publish target: ~/.config/stet/config.toml ($STET_CONFIG_DIR).\n\
                  A repo's own settings: .stet.toml at its root (stet help config)."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Format, spell-check and lint a repo, then apply the house rules. Exits 1 on any error.
    Check(CheckArgs),
    /// Write the shared tool configs into a repo from its .stet.toml.
    Sync(RepoArgs),
    /// Print the completion script for a shell.
    Completion { shell: Shell },
    /// Print the usage spec (KDL) that describes every command and flag.
    Usage,
    /// Topics: config, rules. Or a command's help.
    Help(HelpArgs),
}

#[derive(Debug, clap::Args)]
struct HelpArgs {
    /// A topic (config, rules) or a command.
    topic: Option<String>,
}

#[derive(Debug, clap::Args)]
struct JsonFlag {
    /// Print JSON (see Docs/json-contract.md).
    #[arg(long)]
    json: bool,
}

#[derive(Debug, clap::Args)]
struct RepoArgs {
    /// Repos, or any path inside one. Default: the repo holding the current folder.
    #[arg(value_name = "REPO", conflicts_with = "all")]
    repos: Vec<PathBuf>,
    /// Every repo in ~/.config/stet/config.toml.
    #[arg(long)]
    all: bool,
    #[command(flatten)]
    output: JsonFlag,
}

#[derive(Debug, clap::Args)]
struct CheckArgs {
    #[command(flatten)]
    repos: RepoArgs,
    /// Never change a file: Prettier only checks, as in CI.
    #[arg(long)]
    check_only: bool,
    /// Skip Prettier, cspell, markdownlint and the repo's tasks: config and house rules only.
    #[arg(long)]
    rules_only: bool,
    /// List every warning, not just the count.
    #[arg(long)]
    warnings: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Shell {
    Fish,
    Zsh,
    Bash,
}

impl Command {
    fn json(&self) -> bool {
        match self {
            Command::Check(a) => a.repos.output.json,
            Command::Sync(a) => a.output.json,
            Command::Completion { .. } | Command::Usage | Command::Help(_) => false,
        }
    }
}

/// The whole program. Exit codes: 0 success, 1 failure, 64 usage.
pub fn main() -> ExitCode {
    CompleteEnv::with_factory(Cli::command)
        .completer("stet")
        .complete();
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            use clap::error::ErrorKind;
            let _ = e.print();
            return match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => ExitCode::SUCCESS,
                _ => ExitCode::from(64),
            };
        }
    };
    let json = cli.command.json();
    match run(cli.command) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            let message = error
                .chain()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(": ");
            if json {
                print_json(&ErrorReport {
                    schema: SCHEMA,
                    error: message,
                    reasons: vec![],
                });
            } else {
                eprintln!("stet: {message}");
            }
            ExitCode::from(1)
        }
    }
}

fn run(command: Command) -> Result<u8> {
    let env = Env::from_process();
    match command {
        Command::Check(a) => check_command(&env, a),
        Command::Sync(a) => sync_command(&env, a),
        Command::Completion { shell } => {
            print!("{}", completion(shell)?);
            Ok(0)
        }
        Command::Help(a) => help_command(a),
        Command::Usage => {
            let mut out = vec![];
            clap_usage::generate(&mut Cli::command(), "stet", &mut out);
            std::io::stdout().write_all(&out).into_diagnostic()?;
            Ok(0)
        }
    }
}

fn print_json(value: &impl Serialize) {
    let text = serde_json::to_string_pretty(value).expect("contract types serialize");
    println!("{text}");
}

/// The repos a command works on: `--all`, the paths given, or the current folder's.
fn repos(env: &Env, args: &RepoArgs) -> Result<Vec<Repo>> {
    if args.all {
        let global = Global::load(env)?;
        if global.repos.is_empty() {
            return Err(miette!("no repos in {}", global.abbreviate(&global.file())));
        }
        return global.repos.iter().map(|r| Repo::load(r)).collect();
    }
    if args.repos.is_empty() {
        let cwd = std::env::current_dir().into_diagnostic()?;
        return Ok(vec![Repo::find(&cwd)?]);
    }
    args.repos.iter().map(|p| Repo::find(p)).collect()
}

fn check_command(env: &Env, args: CheckArgs) -> Result<u8> {
    let tools = Tools::new(env);
    let options = Options {
        write: !args.check_only,
        tools: !args.rules_only,
    };
    let mut report = CheckReport {
        schema: SCHEMA,
        ok: true,
        errors: 0,
        warnings: 0,
        repos: vec![],
    };
    for repo in repos(env, &args.repos)? {
        let result = check::check(&repo, &tools, options);
        report.errors += result.errors;
        report.warnings += result.warnings;
        report.repos.push(result);
    }
    report.ok = report.errors == 0;
    if args.repos.output.json {
        print_json(&report);
    } else {
        print_check(&report, args.warnings);
    }
    Ok(if report.ok { 0 } else { 1 })
}

fn print_check(report: &CheckReport, all_warnings: bool) {
    for repo in &report.repos {
        let counts = repo
            .tools
            .iter()
            .filter_map(|t| t.files.map(|n| format!("{} {n}", t.name)))
            .collect::<Vec<_>>();
        let files = if counts.is_empty() {
            String::new()
        } else {
            format!(" (files read: {})", counts.join(", "))
        };
        let verdict = match (repo.errors, repo.warnings) {
            (0, 0) => "clean".to_string(),
            (0, w) => format!("clean, {w} warning{}", plural(w)),
            (e, 0) => format!("{e} error{}", plural(e)),
            (e, w) => format!("{e} error{}, {w} warning{}", plural(e), plural(w)),
        };
        println!("{}: {verdict}{files}", repo.name);
        for file in &repo.formatted {
            println!("  formatted {file}");
        }
        for f in &repo.findings {
            if f.severity == Severity::Warning && !all_warnings {
                continue;
            }
            let at = match (&f.file, f.line) {
                (Some(file), Some(line)) => format!("{file}:{line}"),
                (Some(file), None) => file.clone(),
                _ => f.tool.clone(),
            };
            println!("  {at}  {} {}: {}", f.severity.as_str(), f.rule, f.message);
        }
    }
    if report.warnings > 0 && !all_warnings {
        println!("`stet check --warnings --rules-only` lists the warnings");
    }
}

fn sync_command(env: &Env, args: RepoArgs) -> Result<u8> {
    let mut report = SyncReport {
        schema: SCHEMA,
        repos: vec![],
    };
    for repo in repos(env, &args)? {
        report.repos.push(sync::sync(&repo)?);
    }
    if args.output.json {
        print_json(&report);
    } else {
        for repo in &report.repos {
            let written: Vec<_> = repo
                .files
                .iter()
                .filter(|f| f.action == SyncAction::Written)
                .map(|f| f.path.as_str())
                .collect();
            if written.is_empty() {
                println!("{}: in sync", repo.name);
            } else {
                println!("{}: wrote {}", repo.name, written.join(", "));
            }
        }
    }
    Ok(0)
}

fn help_command(args: HelpArgs) -> Result<u8> {
    let Some(name) = args.topic else {
        let mut command = Cli::command();
        print!("{}", command.render_long_help());
        println!("\nTopics (stet help <topic>):");
        for (name, about, _) in help::TOPICS {
            println!("  {name:<8} {about}");
        }
        return Ok(0);
    };
    if let Some(text) = help::topic(&name) {
        print!("{text}");
        return Ok(0);
    }
    let mut command = Cli::command();
    match command.find_subcommand_mut(&name) {
        Some(sub) => {
            print!("{}", sub.render_long_help());
            Ok(0)
        }
        None => Err(miette!(
            "no help topic or command {name}; topics: config, rules"
        )),
    }
}

fn completion(shell: Shell) -> Result<String> {
    let mut out = vec![];
    let (var, name, bin, completer) = ("COMPLETE", "stet", "stet", "stet");
    let result = match shell {
        Shell::Fish => Fish.write_registration(var, name, bin, completer, &mut out),
        Shell::Zsh => Zsh.write_registration(var, name, bin, completer, &mut out),
        Shell::Bash => Bash.write_registration(var, name, bin, completer, &mut out),
    };
    result.into_diagnostic()?;
    String::from_utf8(out).into_diagnostic()
}

pub fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}
