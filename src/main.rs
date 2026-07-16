mod change;
mod git;
mod session;

use std::{
    collections::HashMap,
    io::{IsTerminal, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::env::{EnvCompleter, Fish as FishCompleter, Zsh as ZshCompleter};
use crossterm::{
    QueueableCommand,
    cursor::{MoveToColumn, MoveUp},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};

use crate::{
    git::{Git, WorktreeState},
    session::Session,
};

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
    /// Create a change worktree
    ///
    /// Managed Pi makes an additional, asynchronous provider request from the
    /// first prompt to infer a title. `--shell` skips Pi and title inference.
    New {
        /// Start the change from this revision (`@` means the invoking worktree)
        #[arg(long, value_name = "REF")]
        from: Option<String>,
        /// Enter the worktree without opening its agent
        #[arg(long)]
        shell: bool,
    },
    /// Open an active change
    Switch {
        /// Enter the worktree without opening its agent
        #[arg(long)]
        shell: bool,
    },
    /// List the repository's active changes
    List,
    /// Remove an active change
    #[command(visible_alias = "delete")]
    Remove {
        /// Discard changes and delete an unmerged branch
        #[arg(long)]
        force: bool,
    },
    /// Print shell integration and completions
    Init { shell: Shell },
    #[command(name = "__title", hide = true)]
    Title {
        #[arg(long)]
        change: String,
        #[arg(long)]
        session: String,
    },
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
        Cmd::New { from, shell } => new(&Git::discover()?, from.as_deref(), shell),
        Cmd::Switch { shell } => switch(&Git::discover()?, shell),
        Cmd::List => list(&Git::discover()?),
        Cmd::Remove { force } => remove(&Git::discover()?, force),
        Cmd::Init { shell } => init(shell),
        Cmd::Title { change, session } => title(&change, &session),
    }
}

fn title(change_id: &str, session_id: &str) -> Result<()> {
    let capsule = std::env::var_os("GROVE_CHANGE_CAPSULE")
        .map(PathBuf::from)
        .context("GROVE_CHANGE_CAPSULE is not set")?;
    let mut prompt = String::new();
    std::io::stdin()
        .read_to_string(&mut prompt)
        .context("failed to read the title prompt")?;
    println!(
        "{}",
        session::infer_title(&capsule, change_id, session_id, &prompt)?
    );
    Ok(())
}

fn new(git: &Git, from: Option<&str>, shell: bool) -> Result<()> {
    if shell {
        require_shell_navigation()?;
    } else {
        Session::prepare()?;
    }
    let change = git.create_change(from)?;
    let path = change.worktree();
    eprintln!("✓ Created {} at {}", change.id, path.display());
    if shell {
        navigate(&path)
    } else {
        Session::for_worktree(&path)?.attach()
    }
}

fn switch(git: &Git, shell: bool) -> Result<()> {
    if shell {
        require_shell_navigation()?;
    }
    let selected = pick(git)?;
    eprintln!(
        "✓ Using {} at {}",
        selected.title_label,
        selected.worktree_path.display()
    );
    if shell {
        navigate(&selected.worktree_path)
    } else {
        Session::for_worktree(&selected.worktree_path)?.attach()
    }
}

fn pick(git: &Git) -> Result<Row> {
    let (choices, _) = rows(git)?;
    if choices.is_empty() {
        bail!("no active changes to switch to");
    }
    let stderr = std::io::stderr();
    if !std::io::stdin().is_terminal() || !stderr.is_terminal() {
        bail!("interactive worktree selection requires a terminal");
    }
    let mut output = stderr.lock();
    select(&mut output, &choices)
}

fn select(output: &mut impl Write, choices: &[Row]) -> Result<Row> {
    let mut raw_mode = RawMode::enter()?;
    let selection = select_raw(output, choices);
    raw_mode.restore()?;
    selection
}

fn select_raw(output: &mut impl Write, choices: &[Row]) -> Result<Row> {
    let mut filter = String::new();
    let mut visible = filtered_rows(choices, &filter);
    print_picker(&visible, &filter, 0, output)?;
    output.flush()?;
    let mut selected: usize = 0;
    loop {
        let Event::Key(key) = event::read().context("read picker input")? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        let next = match key.code {
            KeyCode::Up => selected.saturating_sub(1),
            KeyCode::Down => (selected + 1).min(visible.len().saturating_sub(1)),
            KeyCode::Enter if !visible.is_empty() => return Ok(visible[selected].clone()),
            KeyCode::Esc => bail!("selection cancelled"),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                bail!("selection cancelled")
            }
            KeyCode::Backspace if !filter.is_empty() => {
                filter.pop();
                visible = replace_picker(output, visible.len(), choices, &filter, 0)?;
                selected = 0;
                continue;
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                filter.push(character);
                visible = replace_picker(output, visible.len(), choices, &filter, 0)?;
                selected = 0;
                continue;
            }
            _ => continue,
        };
        if next != selected {
            visible = replace_picker(output, visible.len(), choices, &filter, next)?;
            selected = next;
        }
    }
}

fn filtered_rows(choices: &[Row], filter: &str) -> Vec<Row> {
    let filter = filter.to_lowercase();
    choices
        .iter()
        .filter(|row| row.title_label.to_lowercase().contains(&filter))
        .cloned()
        .collect()
}

fn print_picker(
    rows: &[Row],
    filter: &str,
    selected: usize,
    output: &mut impl Write,
) -> std::io::Result<()> {
    writeln!(output, "Filter: {filter}\r")?;
    print_rows(rows, output, true, "\r\n", Some(selected))
}

fn replace_picker(
    output: &mut impl Write,
    previous_rows: usize,
    choices: &[Row],
    filter: &str,
    selected: usize,
) -> std::io::Result<Vec<Row>> {
    let distance = u16::try_from(previous_rows + 2).unwrap_or(u16::MAX);
    output
        .queue(MoveUp(distance))?
        .queue(MoveToColumn(0))?
        .queue(Clear(ClearType::FromCursorDown))?;
    let rows = filtered_rows(choices, filter);
    print_picker(&rows, filter, selected, output)?;
    output.flush()?;
    Ok(rows)
}

struct RawMode {
    active: bool,
}

impl RawMode {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("enable raw mode for worktree picker")?;
        Ok(Self { active: true })
    }

    fn restore(&mut self) -> Result<()> {
        disable_raw_mode().context("restore terminal mode after worktree picker")?;
        self.active = false;
        Ok(())
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        if self.active {
            let _ = disable_raw_mode();
        }
    }
}

fn list(git: &Git) -> Result<()> {
    let (rows, changed) = rows(git)?;
    let stdout = std::io::stdout();
    let terminal = stdout.is_terminal();
    let mut output = stdout.lock();
    print_rows(&rows, &mut output, terminal, "\n", None)?;
    output.flush()?;
    eprint!("\n○ Showing {} changes", rows.len());
    if changed > 0 {
        eprint!(", {changed} with changes");
    }
    eprintln!();
    Ok(())
}

fn rows(git: &Git) -> Result<(Vec<Row>, usize)> {
    let worktrees = git.inventory()?;
    let current = git.current_path()?;
    let mut title_counts = HashMap::new();
    for worktree in &worktrees {
        if let Some(title) = &worktree.title {
            *title_counts.entry(title.as_str()).or_insert(0_usize) += 1;
        }
    }
    let mut rows = Vec::new();
    let mut changed = 0;
    for worktree in &worktrees {
        let short_id = &worktree.id[..8];
        let title_label = match &worktree.title {
            Some(title) if title_counts.get(title.as_str()) == Some(&1) => title.clone(),
            Some(title) => format!("{title} · {short_id}"),
            None => format!("Untitled · {short_id}"),
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
            current: worktree.current,
            id: worktree.id.clone(),
            worktree_path: worktree.path.clone(),
            title_label,
            base: worktree.base.clone(),
            changes,
            divergence: worktree
                .divergence
                .as_ref()
                .map(format_divergence)
                .unwrap_or_default(),
            path: display_path(&worktree.path, &current),
        });
    }
    Ok((rows, changed))
}

#[derive(Clone)]
struct Row {
    current: bool,
    id: String,
    worktree_path: PathBuf,
    title_label: String,
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

fn print_rows(
    rows: &[Row],
    output: &mut impl Write,
    terminal: bool,
    newline: &str,
    selected: Option<usize>,
) -> std::io::Result<()> {
    let marker_width = 1;
    let title_width = width(rows, "Title", |row| &row.title_label);
    let base_width = width(rows, "Base", |row| &row.base);
    let changes_width = width(rows, "Changes", |row| &row.changes);
    let divergence_width = width(rows, "Base↕", |row| &row.divergence);
    let header = format!(
        "{:<marker_width$} {:<title_width$}  {:<base_width$}  {:<changes_width$}  {:<divergence_width$}  Path",
        "", "Title", "Base", "Changes", "Base↕"
    );
    write!(output, "{}{newline}", bold(&header, terminal))?;
    for (index, row) in rows.iter().enumerate() {
        let marker = if let Some(selected) = selected {
            if index == selected { '›' } else { ' ' }
        } else if row.current {
            '@'
        } else {
            '+'
        };
        let base = format!("{:<base_width$}", row.base);
        let changes = format!("{:<changes_width$}", row.changes);
        let divergence = format!("{:<divergence_width$}", row.divergence);
        write!(
            output,
            "{:<marker_width$} {:<title_width$}  {base}  {changes}  {divergence}  {}{newline}",
            marker, row.title_label, row.path,
        )?;
    }
    Ok(())
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

fn remove(git: &Git, force: bool) -> Result<()> {
    let recovered = git.recover_closing_removals()?;
    if recovered > 0 {
        eprintln!("✓ Finished {recovered} interrupted removal(s)");
        return Ok(());
    }
    let current = git.current_path()?;
    let (rows, _) = rows(git)?;
    let selected = if let Some(current) = rows.into_iter().find(|row| row.current) {
        current
    } else if current == git.primary_path()? {
        pick(git)?
    } else {
        bail!("current worktree is not a managed Grove change");
    };
    if selected.worktree_path == current {
        require_shell_navigation()?;
    }
    let session = Session::for_worktree(&selected.worktree_path)?;
    let _lock = session.lock()?;
    let prepared = git.prepare_removal(&selected.id, force)?;
    let removal = git.remove(prepared)?;
    eprintln!("✓ Removed {}", selected.title_label);
    if let Some(path) = removal.navigate_to {
        navigate(&path)?;
    }
    Ok(())
}

fn navigate(path: &Path) -> Result<()> {
    let file = shell_navigation_file()?;
    std::fs::write(&file, path.as_os_str().as_encoded_bytes()).with_context(|| {
        format!(
            "failed to write shell navigation directive {}",
            file.display()
        )
    })?;
    Ok(())
}

fn require_shell_navigation() -> Result<()> {
    let file = shell_navigation_file()?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .with_context(|| {
            format!(
                "failed to open shell navigation directive {}",
                file.display()
            )
        })?;
    Ok(())
}

fn shell_navigation_file() -> Result<PathBuf> {
    std::env::var_os("GROVE_DIRECTIVE_CD_FILE")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .context(
            "shell integration is not loaded; add `grove init fish | source` or `eval \"$(grove init zsh)\"` to your shell configuration",
        )
}
