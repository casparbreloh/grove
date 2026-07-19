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
    cursor::{Hide, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
    /// Create a Change workspace
    ///
    /// Managed Pi makes an additional, asynchronous provider request from the
    /// first prompt to infer a title. `--shell` skips Pi and title inference.
    New {
        /// Start the change from this revision (`@` means the invoking worktree)
        #[arg(long, value_name = "REF")]
        from: Option<String>,
        /// Enter the workspace without opening its agent
        #[arg(long)]
        shell: bool,
    },
    /// Open a Change or Main
    Switch {
        /// Enter the workspace without opening its agent
        #[arg(long)]
        shell: bool,
    },
    /// List Main and active Changes
    List,
    /// Fetch upstream, archive integrated Changes, and rebase eligible Changes
    Sync,
    /// Archive an active Change
    Archive {
        /// Archive and discard unmerged work
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
        Cmd::Sync => sync(&Git::discover()?),
        Cmd::Archive { force } => archive(&Git::discover()?, force),
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
    let path = change.workspace();
    eprintln!("✓ Created {} at {}", change.id, path.display());
    if shell {
        navigate(&path)
    } else {
        Session::for_workspace(&path)?.attach()
    }
}

fn switch(git: &Git, shell: bool) -> Result<()> {
    if shell {
        require_shell_navigation()?;
    }
    let (mut choices, _) = change_rows(git)?;
    if choices.iter().any(|row| row.current) {
        choices.insert(0, main_row(git)?);
    }
    let Some(selected) = pick(choices)? else {
        return Ok(());
    };
    eprintln!(
        "✓ Using {} at {}",
        selected.title_label,
        selected.worktree_path.display()
    );
    if shell || selected.change_id.is_none() {
        navigate(&selected.worktree_path)
    } else {
        Session::for_workspace(&selected.worktree_path)?.attach()
    }
}

fn pick(choices: Vec<Row>) -> Result<Option<Row>> {
    if choices.is_empty() {
        bail!("no active changes to switch to");
    }
    let stderr = std::io::stderr();
    if !std::io::stdin().is_terminal() || !stderr.is_terminal() {
        bail!("interactive Change selection requires a terminal");
    }
    let mut output = stderr.lock();
    select(&mut output, &choices)
}

fn select(output: &mut impl Write, choices: &[Row]) -> Result<Option<Row>> {
    let mut mode = PickerMode::enter(output)?;
    let selection = select_raw(mode.output(), choices);
    mode.restore()?;
    selection
}

fn select_raw(output: &mut impl Write, choices: &[Row]) -> Result<Option<Row>> {
    print_picker(choices, 0, output)?;
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
            KeyCode::Down => (selected + 1).min(choices.len().saturating_sub(1)),
            KeyCode::Enter => return Ok(Some(choices[selected].clone())),
            KeyCode::Esc => return Ok(None),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(None);
            }
            _ => continue,
        };
        if next != selected {
            redraw_picker(output, choices, next)?;
            selected = next;
        }
    }
}

fn print_picker(rows: &[Row], selected: usize, output: &mut impl Write) -> std::io::Result<()> {
    print_rows(rows, output, true, "\r\n", Some(selected))
}

fn redraw_picker(output: &mut impl Write, rows: &[Row], selected: usize) -> std::io::Result<()> {
    let distance = u16::try_from(rows.len() + 1).unwrap_or(u16::MAX);
    output
        .queue(MoveUp(distance))?
        .queue(MoveToColumn(0))?
        .queue(Clear(ClearType::FromCursorDown))?;
    print_picker(rows, selected, output)?;
    output.flush()
}

struct PickerMode<'a, W: Write> {
    output: &'a mut W,
    active: bool,
}

impl<'a, W: Write> PickerMode<'a, W> {
    fn enter(output: &'a mut W) -> Result<Self> {
        enable_raw_mode().context("enable raw mode for worktree picker")?;
        if let Err(error) = output.queue(Hide).and_then(|output| output.flush()) {
            let _ = output.queue(Show).and_then(|output| output.flush());
            let _ = disable_raw_mode();
            return Err(error).context("hide cursor for worktree picker");
        }
        Ok(Self {
            output,
            active: true,
        })
    }

    fn output(&mut self) -> &mut W {
        self.output
    }

    fn restore(&mut self) -> Result<()> {
        self.output
            .queue(Show)
            .and_then(|output| output.flush())
            .context("restore cursor after worktree picker")?;
        disable_raw_mode().context("restore terminal mode after worktree picker")?;
        self.active = false;
        Ok(())
    }
}

impl<W: Write> Drop for PickerMode<'_, W> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.output.queue(Show).and_then(|output| output.flush());
            let _ = disable_raw_mode();
        }
    }
}

fn sync(git: &Git) -> Result<()> {
    let result = git.sync()?;
    let mut title_counts = HashMap::new();
    for entry in &result.entries {
        if let Some(title) = &entry.title {
            *title_counts.entry(title.as_str()).or_insert(0_usize) += 1;
        }
    }
    let rows = result
        .entries
        .iter()
        .map(|entry| {
            let short_id = &entry.id[..8];
            let title = match &entry.title {
                Some(title) if title_counts.get(title.as_str()) == Some(&1) => title.clone(),
                Some(title) => format!("{title} · {short_id}"),
                None => format!("Untitled · {short_id}"),
            };
            let marker = match entry.outcome.as_str() {
                "archived" => '-',
                "rebased" => '↑',
                "skipped" => '○',
                _ => ' ',
            };
            (marker, title, entry.outcome.as_str(), entry.reason.as_str())
        })
        .collect::<Vec<_>>();
    let title_width = rows
        .iter()
        .map(|(_, title, _, _)| UnicodeWidthStr::width(title.as_str()))
        .max()
        .unwrap_or(0);
    let outcome_width = rows
        .iter()
        .map(|(_, _, outcome, _)| UnicodeWidthStr::width(*outcome))
        .max()
        .unwrap_or(0);
    let stderr = std::io::stderr();
    let max_width = stderr
        .is_terminal()
        .then(|| terminal::size().ok())
        .flatten()
        .map(|(columns, _)| usize::from(columns.saturating_sub(1)));
    let mut output = stderr.lock();
    for (marker, title, outcome, reason) in rows {
        let title_padding = title_width.saturating_sub(UnicodeWidthStr::width(title.as_str()));
        let outcome_padding = outcome_width.saturating_sub(UnicodeWidthStr::width(outcome));
        let line = format!(
            "{marker} {title}{}  {outcome}{}  {reason}",
            " ".repeat(title_padding),
            " ".repeat(outcome_padding)
        );
        writeln!(output, "{}", fit_width(line, max_width))?;
    }
    if !result.entries.is_empty() {
        writeln!(output)?;
    }
    writeln!(
        output,
        "✓ Synced {} Changes: {} archived, {} rebased, {} skipped",
        result.entries.len(),
        result.archived,
        result.rebased,
        result.skipped
    )?;
    output.flush()?;
    Ok(())
}

fn list(git: &Git) -> Result<()> {
    let (mut rows, changed) = change_rows(git)?;
    let changes = rows.len();
    rows.insert(0, main_row(git)?);
    let stdout = std::io::stdout();
    let terminal = stdout.is_terminal();
    let mut output = stdout.lock();
    print_rows(&rows, &mut output, terminal, "\n", None)?;
    output.flush()?;
    eprint!("\n○ Showing {changes} changes");
    if changed > 0 {
        eprint!(", {changed} with changes");
    }
    eprintln!();
    Ok(())
}

fn change_rows(git: &Git) -> Result<(Vec<Row>, usize)> {
    let worktrees = git.inventory()?;
    let current = git.current_path()?;
    let mut title_counts = HashMap::from([("Main", 1_usize)]);
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
            change_id: Some(worktree.id.clone()),
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

fn main_row(git: &Git) -> Result<Row> {
    let current = git.current_path()?;
    let primary = git.primary_path()?;
    Ok(Row {
        current: current == primary,
        change_id: None,
        worktree_path: primary.clone(),
        title_label: "Main".to_owned(),
        base: String::new(),
        changes: String::new(),
        divergence: String::new(),
        path: display_path(&primary, &current),
    })
}

#[derive(Clone)]
struct Row {
    current: bool,
    change_id: Option<String>,
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
    is_terminal: bool,
    newline: &str,
    selected: Option<usize>,
) -> std::io::Result<()> {
    let max_width = is_terminal
        .then(|| terminal::size().ok())
        .flatten()
        .map(|(columns, _)| usize::from(columns.saturating_sub(1)));
    let marker_width = 1;
    let title_width = width(rows, "Title", |row| &row.title_label);
    let base_width = width(rows, "Base", |row| &row.base);
    let changes_width = width(rows, "Changes", |row| &row.changes);
    let divergence_width = width(rows, "Base↕", |row| &row.divergence);
    let header = format!(
        "{:<marker_width$} {:<title_width$}  {:<base_width$}  {:<changes_width$}  {:<divergence_width$}  Path",
        "", "Title", "Base", "Changes", "Base↕"
    );
    let header = fit_width(header, max_width);
    write!(output, "{}{newline}", bold(&header, is_terminal))?;
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
        let line = format!(
            "{:<marker_width$} {:<title_width$}  {base}  {changes}  {divergence}  {}",
            marker, row.title_label, row.path,
        );
        write!(output, "{}{newline}", fit_width(line, max_width))?;
    }
    Ok(())
}

fn fit_width(mut value: String, max_width: Option<usize>) -> String {
    let Some(max_width) = max_width else {
        return value;
    };
    value.retain(|character| UnicodeWidthChar::width(character).is_some());
    if UnicodeWidthStr::width(value.as_str()) <= max_width {
        return value;
    }
    if max_width == 0 {
        return String::new();
    }

    let mut fitted = String::new();
    let mut width = 0;
    for character in value.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if width + character_width + 1 > max_width {
            break;
        }
        fitted.push(character);
        width += character_width;
    }
    fitted.push('…');
    fitted
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

fn archive(git: &Git, force: bool) -> Result<()> {
    let recovered = git.recover_closing_archives()?;
    if recovered > 0 {
        eprintln!("✓ Finished {recovered} interrupted archive(s)");
        return Ok(());
    }
    let current = git.current_path()?;
    let (rows, _) = change_rows(git)?;
    let selected = if let Some(current) = rows.iter().find(|row| row.current) {
        Some(current.clone())
    } else if current == git.primary_path()? {
        pick(rows)?
    } else {
        bail!("current workspace is not a managed Grove Change");
    };
    let Some(selected) = selected else {
        return Ok(());
    };
    if selected.worktree_path == current {
        require_shell_navigation()?;
    }
    let session = Session::for_workspace(&selected.worktree_path)?;
    let _lock = session.lock()?;
    let change_id = selected
        .change_id
        .as_deref()
        .context("selected destination is not a Change")?;
    let prepared = git.prepare_archive(change_id, force)?;
    let archive = git.finish_archive(prepared)?;
    eprintln!("✓ Archived {}", selected.title_label);
    if let Some(path) = archive.navigate_to {
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
