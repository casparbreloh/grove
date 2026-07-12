mod git;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

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
        branch: String,
    },
    /// List the repository's worktrees
    List,
    /// Remove a linked worktree
    Remove { branch: Option<String> },
    /// Print shell integration
    Shell { shell: Shell },
}

#[derive(Clone, ValueEnum)]
enum Shell {
    Zsh,
}

fn main() -> Result<()> {
    if std::env::args_os().len() == 2
        && std::env::args_os().nth(1).as_deref() == Some("--usage-spec".as_ref())
    {
        clap_usage::generate(&mut Cli::command(), "grove", &mut std::io::stdout());
        return Ok(());
    }

    match Cli::parse().command {
        Cmd::Switch { create, branch } => switch(&Git::discover()?, &branch, create),
        Cmd::List => list(&Git::discover()?),
        Cmd::Remove { branch } => remove(&Git::discover()?, branch.as_deref()),
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
        println!("{}", worktree.path.display());
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

    eprintln!("◎ Creating worktree for {branch}…");
    if create {
        let base = git.default_branch()?;
        git.worktree_add_new(&path, branch, &base)?;
    } else {
        git.worktree_add(&path, branch)?;
    }
    eprintln!("✓ Created {branch} at {}", path.display());
    println!("{}", path.display());
    Ok(())
}

fn list(git: &Git) -> Result<()> {
    let worktrees = git.worktrees()?;
    let current = git.current_root()?;
    let primary = primary(&worktrees)?;
    println!("  BRANCH  STATUS  PATH");
    for worktree in &worktrees {
        let marker = if worktree.path == current {
            '@'
        } else if worktree.path == primary.path {
            '^'
        } else {
            '+'
        };
        let branch = worktree.branch().unwrap_or("(detached)");
        let status = if worktree.prunable {
            "prunable"
        } else if git.is_dirty(&worktree.path)? {
            "dirty"
        } else {
            "clean"
        };
        println!("{marker} {branch}  {status}  {}", worktree.path.display());
    }
    println!();
    println!("@ current  ^ primary  + linked");
    Ok(())
}

fn remove(git: &Git, requested: Option<&str>) -> Result<()> {
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
    if target.locked {
        bail!("worktree is locked: {}", target.path.display());
    }
    if git.is_dirty(&target.path)? {
        bail!(
            "worktree has uncommitted changes: {}",
            target.path.display()
        );
    }

    let branch = target.branch().unwrap_or("detached");
    eprintln!("◎ Removing worktree for {branch}…");
    git.worktree_remove(&target.path)?;
    eprintln!("✓ Removed {branch}");
    if target.path == current {
        println!("{}", primary.path.display());
    }
    Ok(())
}

fn primary(worktrees: &[Worktree]) -> Result<&Worktree> {
    worktrees.first().context("repository has no worktrees")
}

fn worktree_path(primary: &Path, branch: &str) -> Result<PathBuf> {
    let name = primary
        .file_name()
        .context("primary worktree has no directory name")?;
    let encoded = encode_branch(branch);
    let directory = format!("{}.{}", name.to_string_lossy(), encoded);
    Ok(primary.with_file_name(directory))
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
