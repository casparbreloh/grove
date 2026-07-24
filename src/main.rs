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
    cursor::{Hide, MoveTo, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    git::{Git, WorktreeState},
    session::Session,
};

#[derive(Parser)]
struct Cli {
    #[arg(long, hide = true)]
    usage_spec: bool,

    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a Change workspace and open Pi
    ///
    /// Managed Pi makes an additional, asynchronous provider request from the
    /// first prompt to infer a title.
    New {
        /// Start the change from this revision (`@` means the invoking worktree)
        #[arg(long, value_name = "REF")]
        from: Option<String>,
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
        None => navigator(&Git::discover()?),
        Some(Cmd::New { from }) => new(&Git::discover()?, from.as_deref()),
        Some(Cmd::List) => list(&Git::discover()?),
        Some(Cmd::Sync) => sync(&Git::discover()?),
        Some(Cmd::Archive { force }) => archive(&Git::discover()?, force),
        Some(Cmd::Init { shell }) => init(shell),
        Some(Cmd::Title { change, session }) => title(&change, &session),
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

fn new(git: &Git, from: Option<&str>) -> Result<()> {
    Session::prepare()?;
    let change = git.create_change(from)?;
    let path = change.workspace();
    eprintln!("✓ Created {} at {}", change.id, path.display());
    Session::for_workspace(&path)?.attach()
}

#[derive(Clone)]
enum NavigatorAction {
    Pi(Row),
    Workspace(Row),
    Main,
    NewPi,
    NewWorkspace,
}

fn navigator(git: &Git) -> Result<()> {
    let (rows, _) = change_rows(git)?;
    let Some(action) = navigate_interactively(rows)? else {
        return Ok(());
    };
    match action {
        NavigatorAction::Pi(row) => {
            eprintln!(
                "✓ Using {} at {}",
                row.title_label,
                row.worktree_path.display()
            );
            Session::for_workspace(&row.worktree_path)?.attach()
        }
        NavigatorAction::Workspace(row) => {
            let destination = navigation_destination(git, &row.worktree_path)?;
            navigate(&destination)
        }
        NavigatorAction::Main => {
            let destination = navigation_destination(git, &git.primary_path()?)?;
            navigate(&destination)
        }
        NavigatorAction::NewPi => new(git, None),
        NavigatorAction::NewWorkspace => {
            require_shell_navigation()?;
            let change = git.create_change(None)?;
            let path = change.workspace();
            eprintln!("✓ Created {} at {}", change.id, path.display());
            let destination = navigation_destination(git, &path)?;
            navigate(&destination)
        }
    }
}

fn navigate_interactively(rows: Vec<Row>) -> Result<Option<NavigatorAction>> {
    let stderr = std::io::stderr();
    if !std::io::stdin().is_terminal() || !stderr.is_terminal() {
        bail!("interactive Change navigation requires a terminal");
    }
    let mut output = stderr.lock();
    let mut mode = TerminalMode::enter(&mut output, true)?;
    let action = navigate_raw(mode.output(), &rows);
    mode.restore()?;
    action
}

fn navigate_raw(output: &mut impl Write, rows: &[Row]) -> Result<Option<NavigatorAction>> {
    let mut filter = String::new();
    let mut selected = 0_usize;
    render_navigator(output, rows, &filter, selected)?;
    output.flush()?;
    loop {
        let key = match event::read().context("read navigator input")? {
            Event::Key(key) => key,
            Event::Resize(_, _) => {
                redraw_navigator(output, rows, &filter, selected)?;
                continue;
            }
            _ => continue,
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        let matching = matching_rows(rows, &filter);
        let action = match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(None),
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(Some(NavigatorAction::NewPi));
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(Some(NavigatorAction::NewWorkspace));
            }
            KeyCode::Esc => return Ok(None),
            KeyCode::Home => return Ok(Some(NavigatorAction::Main)),
            KeyCode::Enter => matching
                .get(selected)
                .map(|row| NavigatorAction::Pi((*row).clone())),
            KeyCode::Tab => matching
                .get(selected)
                .map(|row| NavigatorAction::Workspace((*row).clone())),
            KeyCode::Up => {
                selected = selected.saturating_sub(1);
                None
            }
            KeyCode::Down => {
                selected = (selected + 1).min(matching.len().saturating_sub(1));
                None
            }
            KeyCode::Backspace => {
                filter.pop();
                selected = 0;
                None
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                filter.push(character);
                selected = 0;
                None
            }
            _ => continue,
        };
        if let Some(action) = action {
            return Ok(Some(action));
        }
        redraw_navigator(output, rows, &filter, selected)?;
    }
}

fn matching_rows<'a>(rows: &'a [Row], filter: &str) -> Vec<&'a Row> {
    let filter = filter.to_lowercase();
    rows.iter()
        .filter(|row| row.filter_title.to_lowercase().contains(&filter))
        .collect()
}

const NAVIGATOR_FOOTER: [&str; 6] = [
    "Enter Pi",
    "Tab Shell",
    "Home Main",
    "Ctrl-N New + Pi",
    "Ctrl-T New + Shell",
    "Esc/Ctrl-C Cancel",
];

fn navigator_footer(max_width: usize) -> Vec<String> {
    let max_width = max_width.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    for label in NAVIGATOR_FOOTER {
        let added_width = if line.is_empty() { 0 } else { 2 } + UnicodeWidthStr::width(label);
        if !line.is_empty() && UnicodeWidthStr::width(line.as_str()) + added_width > max_width {
            lines.push(line);
            line = String::new();
        }
        if !line.is_empty() {
            line.push_str("  ");
        }
        line.push_str(label);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn navigator_dimensions() -> (usize, usize) {
    terminal::size()
        .map(|(columns, rows)| {
            (
                usize::from(columns.saturating_sub(1)).max(1),
                usize::from(rows),
            )
        })
        .unwrap_or((79, 24))
}

fn render_navigator(
    output: &mut impl Write,
    rows: &[Row],
    filter: &str,
    selected: usize,
) -> Result<()> {
    let (max_width, height) = navigator_dimensions();
    let matching = matching_rows(rows, filter);
    let mut footer = navigator_footer(max_width);
    let reserved = if matching.is_empty() { 3 } else { 4 };
    footer.truncate(height.saturating_sub(reserved));
    let capacity = if matching.is_empty() {
        0
    } else {
        height.saturating_sub(footer.len() + 3).max(1)
    };
    let start = selected.saturating_sub(capacity.saturating_sub(1));
    let shown = matching
        .iter()
        .skip(start)
        .take(capacity)
        .map(|row| (*row).clone())
        .collect::<Vec<_>>();
    let local_selected = selected
        .saturating_sub(start)
        .min(shown.len().saturating_sub(1));
    print_rows(
        &shown,
        output,
        true,
        "\r\n",
        (!shown.is_empty()).then_some(local_selected),
    )?;
    writeln!(
        output,
        "{}\r",
        fit_width(format!("Filter: {filter}"), Some(max_width))
    )?;
    for line in &footer {
        writeln!(output, "{}\r", fit_width(line.clone(), Some(max_width)))?;
    }
    Ok(())
}

fn redraw_navigator(
    output: &mut impl Write,
    rows: &[Row],
    filter: &str,
    selected: usize,
) -> Result<()> {
    output.queue(MoveTo(0, 0))?.queue(Clear(ClearType::All))?;
    render_navigator(output, rows, filter, selected)?;
    output.flush()?;
    Ok(())
}

fn pick(choices: Vec<Row>) -> Result<Option<Row>> {
    if choices.is_empty() {
        bail!("no active changes to archive");
    }
    let stderr = std::io::stderr();
    if !std::io::stdin().is_terminal() || !stderr.is_terminal() {
        bail!("interactive Change selection requires a terminal");
    }
    let mut output = stderr.lock();
    select(&mut output, &choices)
}

fn select(output: &mut impl Write, choices: &[Row]) -> Result<Option<Row>> {
    let mut mode = TerminalMode::enter(output, false)?;
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

struct TerminalMode<'a, W: Write> {
    output: &'a mut W,
    alternate_screen: bool,
    active: bool,
}

impl<'a, W: Write> TerminalMode<'a, W> {
    fn enter(output: &'a mut W, alternate_screen: bool) -> Result<Self> {
        enable_raw_mode().context("enable raw mode for terminal selection")?;
        let mode = Self {
            output,
            alternate_screen,
            active: true,
        };
        let opened = if alternate_screen {
            mode.output
                .queue(EnterAlternateScreen)
                .and_then(|output| output.queue(Hide))
                .and_then(|output| output.queue(Clear(ClearType::All)))
                .and_then(|output| output.queue(MoveTo(0, 0)))
                .and_then(|output| output.flush())
        } else {
            mode.output.queue(Hide).and_then(|output| output.flush())
        };
        opened.context("open terminal selection")?;
        Ok(mode)
    }

    fn output(&mut self) -> &mut W {
        self.output
    }

    fn restore(&mut self) -> Result<()> {
        let screen = self.restore_screen();
        let terminal = disable_raw_mode().context("restore terminal mode after selection");
        if screen.is_ok() && terminal.is_ok() {
            self.active = false;
        }
        screen.and(terminal)
    }

    fn restore_screen(&mut self) -> Result<()> {
        self.output.queue(Show)?;
        if self.alternate_screen {
            self.output.queue(LeaveAlternateScreen)?;
        }
        self.output
            .flush()
            .context("restore screen after terminal selection")
    }
}

impl<W: Write> Drop for TerminalMode<'_, W> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.restore_screen();
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
            filter_title: worktree
                .title
                .clone()
                .unwrap_or_else(|| "Untitled".to_owned()),
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
        filter_title: "Main".to_owned(),
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
    filter_title: String,
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
    let title_width = width(rows, "Title", |row| &row.title_label);
    let column_widths = [
        width(rows, "Base", |row| &row.base),
        width(rows, "Changes", |row| &row.changes),
        width(rows, "Base↕", |row| &row.divergence),
        width(rows, "Path", |row| &row.path),
    ];
    let mut columns = column_widths.len();
    if let Some(max_width) = max_width {
        while columns > 0
            && 2 + title_width
                + column_widths[..columns]
                    .iter()
                    .map(|width| width + 2)
                    .sum::<usize>()
                > max_width
        {
            columns -= 1;
        }
    }
    let secondary_width = column_widths[..columns]
        .iter()
        .map(|width| width + 2)
        .sum::<usize>();
    let title_width = max_width
        .map(|max_width| title_width.min(max_width.saturating_sub(2 + secondary_width)))
        .unwrap_or(title_width);

    let mut header = format!("  {}", padded("Title", title_width));
    for (index, label) in ["Base", "Changes", "Base↕", "Path"]
        .into_iter()
        .take(columns)
        .enumerate()
    {
        header.push_str("  ");
        header.push_str(&padded(label, column_widths[index]));
    }
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
        let values = [
            row.base.as_str(),
            row.changes.as_str(),
            row.divergence.as_str(),
            row.path.as_str(),
        ];
        let mut line = format!("{marker} {}", fitted_title(row, title_width));
        for (index, value) in values.into_iter().take(columns).enumerate() {
            line.push_str("  ");
            line.push_str(&padded(value, column_widths[index]));
        }
        write!(output, "{}{newline}", fit_width(line, max_width))?;
    }
    Ok(())
}

fn fitted_title(row: &Row, width: usize) -> String {
    let fitted = row
        .change_id
        .as_deref()
        .map(|id| format!(" · {id}"))
        .filter(|suffix| row.title_label.ends_with(suffix))
        .and_then(|suffix| {
            let suffix_width = UnicodeWidthStr::width(suffix.as_str());
            (suffix_width <= width).then(|| {
                let title = row.title_label.strip_suffix(&suffix).unwrap_or_default();
                format!(
                    "{}{}",
                    fit_width(title.to_owned(), Some(width - suffix_width)),
                    suffix
                )
            })
        })
        .unwrap_or_else(|| fit_width(row.title_label.clone(), Some(width)));
    padded(&fitted, width)
}

fn padded(value: &str, width: usize) -> String {
    format!(
        "{value}{}",
        " ".repeat(width.saturating_sub(UnicodeWidthStr::width(value)))
    )
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
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
        .max(UnicodeWidthStr::width(header))
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
    let archive_destination = if selected.worktree_path == current {
        require_shell_navigation()?;
        Some(navigation_destination(git, &git.primary_path()?)?)
    } else {
        None
    };
    let session = Session::for_workspace(&selected.worktree_path)?;
    let _lock = session.lock()?;
    let change_id = selected
        .change_id
        .as_deref()
        .context("selected destination is not a Change")?;
    let prepared = git.prepare_archive(change_id, force)?;
    git.finish_archive(prepared)?;
    eprintln!("✓ Archived {}", selected.title_label);
    if let Some(path) = archive_destination {
        navigate(&path)?;
    }
    Ok(())
}

fn navigation_destination(git: &Git, destination_root: &Path) -> Result<PathBuf> {
    if !destination_root.is_dir() {
        bail!("workspace is missing: {}", destination_root.display());
    }
    let current_root = git.current_path()?;
    let invocation = std::env::current_dir().context("failed to resolve current directory")?;
    let relative = invocation
        .strip_prefix(&current_root)
        .unwrap_or(Path::new(""));
    let candidate = destination_root.join(relative);
    Ok(if candidate.is_dir() {
        candidate
    } else {
        destination_root.to_owned()
    })
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
