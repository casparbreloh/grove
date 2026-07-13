mod git;

use std::{
    io::IsTerminal,
    path::{Path, PathBuf},
};

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
    let detected = git.default_branch().or_else(|_| {
        primary
            .branch()
            .map(str::to_owned)
            .context("could not detect the primary branch")
    })?;
    let default_name = detected.rsplit('/').next().unwrap_or(&detected);
    let default = if git.branch_exists(default_name)? {
        default_name
    } else {
        &detected
    };
    let mut rows = Vec::new();
    let mut changed = 0;
    let mut ahead_count = 0;
    for worktree in &worktrees {
        let marker = if worktree.path == current {
            '@'
        } else if worktree.path == primary.path {
            '^'
        } else {
            '+'
        };
        let branch = worktree.branch().unwrap_or("(detached)").to_owned();
        let (status, diff, main, commit, message) = if worktree.prunable {
            (
                "⊟".to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            )
        } else {
            let mut status = git.status(&worktree.path)?;
            if !status.flags.is_empty() {
                changed += 1;
            }
            if worktree.locked {
                status.flags.insert(0, '⊞');
            }
            let (ahead, behind) = git.divergence(&worktree.path, default)?;
            ahead_count += usize::from(ahead > 0);
            let (commit, message) = git.commit(&worktree.path)?;
            (
                status.flags,
                format_diff(status.added, status.deleted),
                format_divergence(ahead, behind),
                commit,
                message,
            )
        };
        rows.push(Row {
            marker,
            branch,
            status,
            diff,
            main,
            path: display_path(&worktree.path, &current),
            commit,
            message,
        });
    }
    print_rows(&rows, default_name);
    eprint!("\n○ Showing {} worktrees", rows.len());
    if changed > 0 {
        eprint!(", {changed} with changes");
    }
    if ahead_count > 0 {
        eprint!(", {ahead_count} ahead");
    }
    eprintln!();
    Ok(())
}

struct Row {
    marker: char,
    branch: String,
    status: String,
    diff: String,
    main: String,
    path: String,
    commit: String,
    message: String,
}

fn print_rows(rows: &[Row], default: &str) {
    let branch_width = width(rows, "Branch", |row| &row.branch);
    let status_width = width(rows, "Status", |row| &row.status);
    let diff_width = width(rows, "HEAD±", |row| &row.diff);
    let main_header = format!("{default}↕");
    let main_width = width(rows, &main_header, |row| &row.main);
    let path_width = width(rows, "Path", |row| &row.path);
    let color = std::io::stdout().is_terminal();

    println!(
        "  {}  {}  {}  {}  {}  {}  Message",
        styled(&format!("{:<branch_width$}", "Branch"), "1", color),
        styled(&format!("{:<status_width$}", "Status"), "1", color),
        styled(&format!("{:<diff_width$}", "HEAD±"), "1", color),
        styled(&format!("{:<main_width$}", main_header), "1", color),
        styled(&format!("{:<path_width$}", "Path"), "1", color),
        styled("Commit", "1", color),
    );
    for row in rows {
        let marker = styled(
            &row.marker.to_string(),
            if row.marker == '@' { "36" } else { "2" },
            color,
        );
        let status = styled(&format!("{:<status_width$}", row.status), "33", color);
        let metadata = |text: String| styled(&text, "2", color);
        println!(
            "{marker} {:<branch_width$}  {status}  {:<diff_width$}  {:<main_width$}  {}  {}  {}",
            row.branch,
            row.diff,
            row.main,
            metadata(format!("{:<path_width$}", row.path)),
            metadata(format!("{:<8}", row.commit)),
            metadata(row.message.clone()),
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

fn styled(text: &str, code: &str, enabled: bool) -> String {
    if enabled && !text.trim().is_empty() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

fn format_diff(added: usize, deleted: usize) -> String {
    match (added, deleted) {
        (0, 0) => String::new(),
        (_, 0) => format!("+{added}"),
        (0, _) => format!("-{deleted}"),
        _ => format!("+{added} -{deleted}"),
    }
}

fn format_divergence(ahead: usize, behind: usize) -> String {
    match (ahead, behind) {
        (0, 0) => "|".to_owned(),
        (_, 0) => format!("↑{ahead}"),
        (0, _) => format!("↓{behind}"),
        _ => format!("↑{ahead} ↓{behind}"),
    }
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
