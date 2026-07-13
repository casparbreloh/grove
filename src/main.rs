mod git;

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};

use crate::git::{Git, Worktree};

#[derive(Parser)]
#[command(arg_required_else_help = true)]
struct Cli {
    /// Emit the CLI's usage specification.
    #[arg(long, hide = true)]
    usage_spec: bool,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Go to a branch's worktree
    Switch {
        /// Create the branch from the default branch
        #[arg(long)]
        create: bool,
        #[arg(add = ArgValueCompleter::new(branches))]
        branch: String,
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
    /// Print shell integration
    Shell { shell: Shell },
}

#[derive(Clone, ValueEnum)]
enum Shell {
    Fish,
    Zsh,
}

fn main() -> Result<()> {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();

    if std::env::args_os().len() == 2
        && std::env::args_os().nth(1).as_deref() == Some("--usage-spec".as_ref())
    {
        clap_usage::generate(&mut Cli::command(), "grove", &mut std::io::stdout());
        return Ok(());
    }

    match Cli::parse().command {
        Cmd::Switch { create, branch } => switch(&Git::discover()?, &branch, create),
        Cmd::List => list(&Git::discover()?),
        Cmd::Remove { force, branch } => remove(&Git::discover()?, branch.as_deref(), force),
        Cmd::Shell { shell: Shell::Fish } => {
            print!("{}", include_str!("shell.fish"));
            Ok(())
        }
        Cmd::Shell { shell: Shell::Zsh } => {
            print!("{}", include_str!("shell.zsh"));
            Ok(())
        }
    }
}

fn switch(git: &Git, branch: &str, create: bool) -> Result<()> {
    git.validate_branch(branch)?;
    let worktrees = git.worktrees()?;
    if let Some(worktree) = worktrees
        .iter()
        .find(|worktree| worktree.branch() == Some(branch))
    {
        if create {
            bail!("branch '{branch}' already exists");
        }
        eprintln!("✓ Using {branch} at {}", worktree.path.display());
        navigate(&worktree.path)?;
        return Ok(());
    }

    let branch_exists = git.branch_exists(branch)?;
    if create && branch_exists {
        bail!("branch '{branch}' already exists");
    }
    if !create && !branch_exists {
        bail!("branch '{branch}' does not exist; create it with --create");
    }

    let primary = primary(&worktrees)?;
    let path = worktree_path(&primary.path, branch)?;
    if path.exists() {
        bail!("worktree path already exists: {}", path.display());
    }
    std::fs::create_dir_all(path.parent().context("worktree path has no parent")?)?;

    eprintln!("◎ Creating worktree for {branch}…");
    if create {
        let base = git.default_branch()?;
        git.worktree_add_new(&path, branch, &base)?;
    } else {
        git.worktree_add(&path, branch)?;
    }
    eprintln!("✓ Created {branch} at {}", path.display());
    navigate(&path)?;
    Ok(())
}

fn list(git: &Git) -> Result<()> {
    let worktrees = git.worktrees()?;
    let current = git.current_root()?;
    let primary = primary(&worktrees)?;
    let mut rows = Vec::new();
    let mut changed = 0;
    for worktree in &worktrees {
        let marker = if worktree.path == current {
            '@'
        } else if worktree.path == primary.path {
            '^'
        } else {
            '+'
        };
        let branch = worktree.branch().unwrap_or("(detached)").to_owned();
        let changes = if worktree.prunable {
            "⊟".to_owned()
        } else {
            let status = git.status(&worktree.path)?;
            if status.changed {
                changed += 1;
            }
            if status.conflicts {
                "×"
            } else if worktree.locked {
                "⊞"
            } else if status.changed {
                "●"
            } else {
                ""
            }
            .to_owned()
        };
        rows.push(Row {
            marker,
            branch,
            changes,
            path: display_path(&worktree.path, &current),
        });
    }
    print_rows(&rows);
    eprint!("\n○ Showing {} worktrees", rows.len());
    if changed > 0 {
        eprint!(", {changed} with changes");
    }
    eprintln!();
    Ok(())
}

struct Row {
    marker: char,
    branch: String,
    changes: String,
    path: String,
}

fn print_rows(rows: &[Row]) {
    let branch_width = width(rows, "Branch", |row| &row.branch);
    let changes_width = width(rows, "Changes", |row| &row.changes);
    println!(
        "  {:<branch_width$}  {:<changes_width$}  Path",
        "Branch", "Changes"
    );
    for row in rows {
        let marker = row.marker;
        let changes = format!("{:<changes_width$}", row.changes);
        println!(
            "{marker} {:<branch_width$}  {changes}  {}",
            row.branch, row.path,
        );
    }
}

fn width<'a>(rows: &'a [Row], header: &str, value: impl Fn(&'a Row) -> &'a str) -> usize {
    rows.iter()
        .map(value)
        .map(str::len)
        .max()
        .unwrap_or(0)
        .max(header.len())
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
    let worktrees = git.worktrees()?;
    let primary = primary(&worktrees)?;
    let current = git.current_root()?;
    let target = match requested {
        Some(branch) => worktrees
            .iter()
            .find(|worktree| worktree.branch() == Some(branch))
            .with_context(|| format!("branch '{branch}' has no worktree"))?,
        None => worktrees
            .iter()
            .find(|worktree| worktree.path == current)
            .context("current directory is not in a worktree")?,
    };
    if target.path == primary.path {
        bail!("cannot remove the primary worktree");
    }
    if target.locked && !force {
        bail!("worktree is locked: {}", target.path.display());
    }
    if !force && git.is_dirty(&target.path)? {
        bail!(
            "worktree has uncommitted changes: {}",
            target.path.display()
        );
    }

    let branch = target.branch();
    let expected_oid = if force {
        None
    } else {
        branch.map(|branch| git.branch_oid(branch)).transpose()?
    };
    if !force && let Some(branch) = branch {
        let detected = git.default_branch()?;
        let name = detected.rsplit('/').next().unwrap_or(&detected);
        let base = if git.branch_exists(name)? {
            name
        } else {
            &detected
        };
        if !git.branch_merged(branch, base)? {
            bail!("branch '{branch}' is not merged; use --force to discard it");
        }
    }

    let label = branch.unwrap_or("detached");
    eprintln!("◎ Removing worktree for {label}…");
    git.worktree_remove(&target.path, force)?;
    if let Some(branch) = branch {
        git.delete_branch(&primary.path, branch, expected_oid.as_deref())?;
    }
    eprintln!("✓ Removed {label}");
    if target.path == current {
        navigate(&primary.path)?;
    }
    Ok(())
}

fn primary(worktrees: &[Worktree]) -> Result<&Worktree> {
    worktrees.first().context("repository has no worktrees")
}

fn worktree_path(primary: &Path, branch: &str) -> Result<PathBuf> {
    let repo = primary
        .file_name()
        .context("primary worktree has no directory name")?;
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home)
        .join(".grove")
        .join(encode_branch(&repo.to_string_lossy()))
        .join(encode_branch(branch)))
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
    let values = if worktrees_only {
        git.worktrees().map(|worktrees| {
            worktrees
                .into_iter()
                .filter_map(|worktree| worktree.branch().map(str::to_owned))
                .collect()
        })
    } else {
        git.branches()
    };
    values
        .unwrap_or_default()
        .into_iter()
        .filter(|value| value.starts_with(current.as_ref()))
        .map(CompletionCandidate::new)
        .collect()
}

fn encode_branch(branch: &str) -> String {
    let mut encoded = String::with_capacity(branch.len());
    for byte in branch.bytes() {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
