mod agent;
mod git;
mod runtime;

use std::{
    ffi::OsStr,
    io::{IsTerminal, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use clap_complete::env::{EnvCompleter, Fish as FishCompleter, Zsh as ZshCompleter};

use crate::agent::Agent;
use crate::git::{Git, WorktreeState};

#[derive(Parser)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[arg(long, hide = true)]
    usage_spec: bool,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Open a persistent agent session
    Agent {
        /// Configured agent name [default: project, global, then pi]
        name: Option<String>,
    },
    /// Go to a worktree
    Switch {
        /// Create a change worktree
        #[arg(short, long)]
        create: bool,
        /// Start a created change from this revision (`@` means the invoking worktree)
        #[arg(long, requires = "create")]
        from: Option<String>,
        /// Change ID or branch, or the title with `--create` [default: choose interactively]
        #[arg(add = ArgValueCompleter::new(branches))]
        target: Option<String>,
    },
    /// List the repository's worktrees
    List,
    /// Remove a linked worktree
    Remove {
        /// Discard changes and delete an unmerged branch
        #[arg(long)]
        force: bool,
        /// Branch to remove [default: current]
        #[arg(add = ArgValueCompleter::new(worktree_branches))]
        branch: Option<String>,
    },
    /// Print shell integration and completions
    Init { shell: Shell },
}

#[derive(Clone, ValueEnum)]
enum Shell {
    Fish,
    Zsh,
}

fn main() -> Result<()> {
    if std::env::args_os().nth(1).as_deref() == Some(rmux_client::INTERNAL_DAEMON_FLAG.as_ref()) {
        return runtime::run_daemon(std::env::args_os().skip(2));
    }
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();

    if std::env::args_os().len() == 2
        && std::env::args_os().nth(1).as_deref() == Some("--usage-spec".as_ref())
    {
        clap_usage::generate(&mut Cli::command(), "grove", &mut std::io::stdout());
        return Ok(());
    }

    match Cli::parse().command {
        Cmd::Agent { name } => {
            let git = Git::discover()?;
            let agent = Agent::load(&git, name.as_deref())?;
            unsafe { std::env::remove_var("GROVE_DIRECTIVE_CD_FILE") };
            agent.attach(&git)
        }
        Cmd::Switch {
            create,
            from,
            target,
        } => switch(
            &Git::discover()?,
            target.as_deref(),
            create,
            from.as_deref(),
        ),
        Cmd::List => list(&Git::discover()?),
        Cmd::Remove { force, branch } => remove(&Git::discover()?, branch.as_deref(), force),
        Cmd::Init { shell } => init(shell),
    }
}

fn switch(git: &Git, target: Option<&str>, create: bool, from: Option<&str>) -> Result<()> {
    if create {
        let change = git.create_change(from, target)?;
        eprintln!("✓ Created {} at {}", change.id, change.path.display());
        return navigate(&change.path);
    }
    let branch = target.map(str::to_owned).map_or_else(|| pick(git), Ok)?;
    let path = git.enter(&branch)?;
    eprintln!("✓ Using {branch} at {}", path.display());
    navigate(&path)
}

fn pick(git: &Git) -> Result<String> {
    let choices = git
        .inventory()?
        .into_iter()
        .filter(|worktree| !worktree.current)
        .filter_map(|worktree| {
            let branch = worktree.branch?;
            let title = if worktree.is_change {
                worktree.title.unwrap_or_else(|| "(untitled)".to_owned())
            } else {
                branch.clone()
            };
            Some((branch, ellipsize(&title, 60)))
        })
        .collect::<Vec<_>>();
    if choices.is_empty() {
        anyhow::bail!("no other worktrees to switch to");
    }
    eprintln!("Select a worktree:");
    for (index, (branch, title)) in choices.iter().enumerate() {
        eprintln!("  {}. {title}  {branch}", index + 1);
    }
    eprint!("> ");
    std::io::stderr().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let index = input
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|index| (1..=choices.len()).contains(index))
        .context("select a listed worktree number")?;
    Ok(choices[index - 1].0.clone())
}

fn list(git: &Git) -> Result<()> {
    let worktrees = git.inventory()?;
    let current = worktrees
        .iter()
        .find(|worktree| worktree.current)
        .map(|worktree| worktree.path.as_path())
        .context("current worktree is missing")?;
    let mut rows = Vec::new();
    let mut changed = 0;
    for worktree in &worktrees {
        let marker = if worktree.current {
            '@'
        } else if worktree.primary {
            '^'
        } else {
            '+'
        };
        let branch = worktree
            .branch
            .as_deref()
            .unwrap_or("(detached)")
            .to_owned();
        let (change, id) = if worktree.is_change {
            (
                worktree
                    .title
                    .as_deref()
                    .map(|title| ellipsize(title, 60))
                    .unwrap_or_else(|| "(untitled)".to_owned()),
                branch,
            )
        } else {
            (branch, String::new())
        };
        let changes = match &worktree.state {
            WorktreeState::Missing => "missing".to_owned(),
            WorktreeState::Present(status) => {
                if status.changed {
                    changed += 1;
                }
                format_changes(status)
            }
        };
        rows.push(Row {
            marker,
            change,
            id,
            base: worktree.base.clone(),
            changes,
            divergence: worktree
                .divergence
                .as_ref()
                .map(format_divergence)
                .unwrap_or_default(),
            path: display_path(&worktree.path, current),
        });
    }
    print_rows(&rows);
    std::io::stdout().flush()?;
    eprint!("\n○ Showing {} worktrees", rows.len());
    if changed > 0 {
        eprint!(", {changed} with changes");
    }
    eprintln!();
    Ok(())
}

struct Row {
    marker: char,
    change: String,
    id: String,
    base: String,
    changes: String,
    divergence: String,
    path: String,
}

fn format_changes(status: &git::Status) -> String {
    let mut parts = Vec::new();
    if status.added > 0 {
        parts.push(format!("+{}", status.added));
    }
    if status.deleted > 0 {
        parts.push(format!("-{}", status.deleted));
    }
    if status.conflicts > 0 {
        let label = if status.conflicts == 1 {
            "conflict"
        } else {
            "conflicts"
        };
        parts.push(format!("{} {label}", status.conflicts));
    }
    parts.join(" ")
}

fn format_divergence(divergence: &git::Divergence) -> String {
    match (divergence.ahead, divergence.behind) {
        (0, 0) => String::new(),
        (ahead, 0) => format!("↑{ahead}"),
        (0, behind) => format!("↓{behind}"),
        (ahead, behind) => format!("↑{ahead} ↓{behind}"),
    }
}

fn print_rows(rows: &[Row]) {
    let change_width = width(rows, "Change", |row| &row.change);
    let id_width = width(rows, "ID", |row| &row.id);
    let base_width = width(rows, "Base", |row| &row.base);
    let changes_width = width(rows, "Changes", |row| &row.changes);
    let divergence_width = width(rows, "Base↕", |row| &row.divergence);
    let header = format!(
        "  {:<change_width$}  {:<id_width$}  {:<base_width$}  {:<changes_width$}  {:<divergence_width$}  Path",
        "Change", "ID", "Base", "Changes", "Base↕"
    );
    println!("{}", bold(&header, std::io::stdout().is_terminal()));
    for row in rows {
        let marker = row.marker;
        let base = format!("{:<base_width$}", row.base);
        let changes = format!("{:<changes_width$}", row.changes);
        let divergence = format!("{:<divergence_width$}", row.divergence);
        println!(
            "{marker} {:<change_width$}  {:<id_width$}  {base}  {changes}  {divergence}  {}",
            row.change, row.id, row.path,
        );
    }
}

fn bold(value: &str, enabled: bool) -> String {
    if enabled {
        format!("\x1b[1m{value}\x1b[0m")
    } else {
        value.to_owned()
    }
}

fn width<'a>(rows: &'a [Row], header: &str, value: impl Fn(&'a Row) -> &'a str) -> usize {
    rows.iter()
        .map(value)
        .map(|value| value.chars().count())
        .max()
        .unwrap_or(0)
        .max(header.chars().count())
}

fn ellipsize(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_owned();
    }
    value.chars().take(width - 1).chain(['…']).collect()
}

fn init(shell: Shell) -> Result<()> {
    let executable = std::env::current_exe()?;
    let executable = executable.to_string_lossy();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    match shell {
        Shell::Fish => {
            FishCompleter.write_registration(
                "COMPLETE",
                "grove",
                "grove",
                &executable,
                &mut output,
            )?;
            output.write_all(include_bytes!("shell.fish"))?;
        }
        Shell::Zsh => {
            ZshCompleter.write_registration(
                "COMPLETE",
                "grove",
                "grove",
                &executable,
                &mut output,
            )?;
            output.write_all(include_bytes!("shell.zsh"))?;
        }
    }
    Ok(())
}

fn display_path(path: &Path, current: &Path) -> String {
    if path == current {
        return ".".to_owned();
    }
    if path.parent() == current.parent() {
        return format!(
            "../{}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from)
        && let Ok(relative) = path.strip_prefix(home)
    {
        return format!("~/{}", relative.display());
    }
    path.display().to_string()
}

fn remove(git: &Git, requested: Option<&str>, force: bool) -> Result<()> {
    let prepared = git.prepare_removal(requested, force)?;
    let sessions = runtime::sessions(prepared.identity())?;
    if !sessions.is_empty() && !force {
        bail!(
            "worktree has {} live agent session{}; use --force to stop them",
            sessions.len(),
            if sessions.len() == 1 { "" } else { "s" }
        );
    }
    if force {
        runtime::terminate(sessions)?;
    }
    let removal = git.remove(prepared)?;
    eprintln!("✓ Removed {}", removal.label);
    if let Some(path) = removal.navigate_to {
        navigate(&path)?;
    }
    Ok(())
}

fn navigate(path: &Path) -> Result<()> {
    if let Some(file) = std::env::var_os("GROVE_DIRECTIVE_CD_FILE") {
        std::fs::write(file, path.as_os_str().as_encoded_bytes())?;
    }
    Ok(())
}

fn branches(current: &OsStr) -> Vec<CompletionCandidate> {
    complete(current, false)
}

fn worktree_branches(current: &OsStr) -> Vec<CompletionCandidate> {
    complete(current, true)
}

fn complete(current: &OsStr, worktrees_only: bool) -> Vec<CompletionCandidate> {
    let current = current.to_string_lossy();
    let Ok(git) = Git::discover() else {
        return Vec::new();
    };
    git.branch_names(worktrees_only)
        .unwrap_or_default()
        .into_iter()
        .filter(|value| value.starts_with(current.as_ref()))
        .map(CompletionCandidate::new)
        .collect()
}
