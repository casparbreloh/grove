use std::{
    io::{IsTerminal, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use crossterm::{
    QueueableCommand,
    cursor::{Hide, MoveToColumn, MoveUp, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    change::{display_safe, display_text, title_labels},
    git::{self, Git, WorktreeState},
    session,
};

enum NavigatorAction {
    Pi(Row),
    Workspace(Row),
    Main,
}

pub(crate) fn run(git: &Git) -> Result<()> {
    let rows = change_rows(git)?;
    let main = main_row(git)?;
    let Some(action) = navigate_interactively(main, rows)? else {
        return Ok(());
    };
    match action {
        NavigatorAction::Pi(row) => {
            row.require_available()?;
            eprintln!(
                "✓ Using {} at {}",
                row.title_label,
                display_text(&row.worktree_path.display().to_string())
            );
            session::attach(&row.worktree_path)
        }
        NavigatorAction::Workspace(row) => {
            row.require_available()?;
            let destination = destination(git, &row.worktree_path)?;
            navigate(&destination)
        }
        NavigatorAction::Main => {
            let destination = destination(git, &git.primary_path()?)?;
            navigate(&destination)
        }
    }
}

pub(crate) fn destination(git: &Git, destination_root: &Path) -> Result<PathBuf> {
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

pub(crate) fn navigate(path: &Path) -> Result<()> {
    let file = shell_navigation_file()?;
    std::fs::write(&file, path.as_os_str().as_encoded_bytes()).with_context(|| {
        format!(
            "failed to write shell navigation directive {}",
            file.display()
        )
    })?;
    Ok(())
}

pub(crate) fn require_shell_navigation() -> Result<()> {
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

fn navigate_interactively(main: Row, rows: Vec<Row>) -> Result<Option<NavigatorAction>> {
    let stderr = std::io::stderr();
    if !std::io::stdin().is_terminal() || !stderr.is_terminal() {
        bail!("interactive Change navigation requires a terminal");
    }
    let mut output = stderr.lock();
    let mut mode = TerminalMode::enter(&mut output)?;
    let mut rendered_lines = 0;
    let action = navigate_raw(mode.output(), &main, &rows, &mut rendered_lines);
    let cleared = clear_rendered(mode.output(), rendered_lines).context("clear navigator");
    let restored = mode.restore();
    cleared?;
    restored?;
    action
}

fn navigate_raw(
    output: &mut impl Write,
    main: &Row,
    rows: &[Row],
    rendered_lines: &mut usize,
) -> Result<Option<NavigatorAction>> {
    let mut selected = 0;
    redraw_navigator(output, main, rows, selected, rendered_lines)?;
    loop {
        let key = match event::read().context("read navigator input")? {
            Event::Key(key) => key,
            Event::Resize(_, _) => {
                redraw_navigator(output, main, rows, selected, rendered_lines)?;
                continue;
            }
            _ => continue,
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        let action = match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(None),
            KeyCode::Esc => return Ok(None),
            KeyCode::Enter => navigator_action(rows, selected, false),
            KeyCode::Tab => navigator_action(rows, selected, true),
            KeyCode::Up => {
                selected = selected.saturating_sub(1);
                None
            }
            KeyCode::Down => {
                selected = (selected + 1).min(rows.len());
                None
            }
            _ => continue,
        };
        if let Some(action) = action {
            return Ok(Some(action));
        }
        redraw_navigator(output, main, rows, selected, rendered_lines)?;
    }
}

fn navigator_action(rows: &[Row], selected: usize, shell: bool) -> Option<NavigatorAction> {
    if selected == 0 {
        return Some(NavigatorAction::Main);
    }
    rows.get(selected - 1).map(|row| {
        if shell {
            NavigatorAction::Workspace(row.clone())
        } else {
            NavigatorAction::Pi(row.clone())
        }
    })
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

fn navigator_styling() -> bool {
    std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
        && std::env::var_os("TERM").is_none_or(|value| value != "dumb")
}

fn redraw_navigator(
    output: &mut impl Write,
    main: &Row,
    rows: &[Row],
    selected: usize,
    rendered_lines: &mut usize,
) -> Result<()> {
    clear_rendered(output, *rendered_lines)?;
    *rendered_lines = 0;
    *rendered_lines = render_navigator(output, main, rows, selected)?;
    output.flush()?;
    Ok(())
}

fn clear_rendered(output: &mut impl Write, rendered_lines: usize) -> std::io::Result<()> {
    if rendered_lines > 0 {
        output
            .queue(MoveUp(u16::try_from(rendered_lines).unwrap_or(u16::MAX)))?
            .queue(MoveToColumn(0))?
            .queue(Clear(ClearType::FromCursorDown))?
            .flush()?;
    }
    Ok(())
}

fn render_navigator(
    output: &mut impl Write,
    main: &Row,
    rows: &[Row],
    selected: usize,
) -> Result<usize> {
    let (max_width, height) = navigator_dimensions();
    if height < 4 {
        bail!("interactive Change navigation requires at least 4 terminal rows");
    }
    let capacity = height - 3;
    let change_selected = selected.saturating_sub(1).min(rows.len().saturating_sub(1));
    let start = change_selected.saturating_sub(capacity.saturating_sub(1));
    let shown = rows.iter().skip(start).take(capacity).collect::<Vec<_>>();
    let styled = navigator_styling();

    let mut visible = Vec::with_capacity(shown.len() + 1);
    visible.push((0, main));
    visible.extend(
        shown
            .iter()
            .enumerate()
            .map(|(index, row)| (start + index + 1, *row)),
    );
    let layout_rows = visible.iter().map(|(_, row)| *row).collect::<Vec<_>>();
    let layout = TableLayout::new(&layout_rows, max_width, 2);
    writeln!(output, "{}\r", bold(&layout.header(), styled))?;
    for (logical_index, row) in visible {
        writeln!(output, "{}\r", layout.row(row, logical_index == selected))?;
    }
    Ok(layout_rows.len() + 1)
}

struct TableLayout {
    leading_width: usize,
    title_width: usize,
    column_widths: [usize; 4],
    columns: usize,
}

impl TableLayout {
    fn new(rows: &[&Row], max_width: usize, leading_width: usize) -> Self {
        let mut title_width = measured_width(rows, "Title", |row| &row.title_label);
        let column_widths = [
            measured_width(rows, "Base", |row| &row.base),
            measured_width(rows, "Changes", |row| &row.changes),
            measured_width(rows, "Base↕", |row| &row.divergence),
            measured_width(rows, "Path", |row| &row.path),
        ];
        let mut columns = column_widths.len();
        while columns > 0
            && leading_width
                + title_width
                + column_widths[..columns]
                    .iter()
                    .map(|width| width + 2)
                    .sum::<usize>()
                > max_width
        {
            columns -= 1;
        }
        let secondary_width = column_widths[..columns]
            .iter()
            .map(|width| width + 2)
            .sum::<usize>();
        title_width = title_width.min(max_width.saturating_sub(leading_width + secondary_width));
        Self {
            leading_width,
            title_width,
            column_widths,
            columns,
        }
    }

    fn header(&self) -> String {
        let mut header = format!(
            "{}{}",
            " ".repeat(self.leading_width),
            padded("Title", self.title_width)
        );
        self.push_columns(&mut header, ["Base", "Changes", "Base↕", "Path"]);
        header
    }

    fn row(&self, row: &Row, selected: bool) -> String {
        let (title, mut metadata) = self.title(row);
        self.push_columns(&mut metadata, row_values(row));
        format!("{} {title}{metadata}", if selected { '›' } else { ' ' })
    }

    fn title(&self, row: &Row) -> (String, String) {
        let suffix = row
            .change_id
            .as_deref()
            .map(|id| format!(" · {id}"))
            .filter(|suffix| row.title_label.ends_with(suffix))
            .unwrap_or_default();
        let suffix_width = UnicodeWidthStr::width(suffix.as_str()).min(self.title_width);
        let title = fit_width(
            row.title_label
                .strip_suffix(&suffix)
                .unwrap_or(&row.title_label)
                .to_owned(),
            Some(self.title_width.saturating_sub(suffix_width)),
        );
        let used = UnicodeWidthStr::width(title.as_str()) + suffix_width;
        let metadata = format!(
            "{}{}",
            fit_width(suffix, Some(suffix_width)),
            " ".repeat(self.title_width.saturating_sub(used))
        );
        (title, metadata)
    }

    fn push_columns<'a>(&self, line: &mut String, values: impl IntoIterator<Item = &'a str>) {
        for (index, value) in values.into_iter().take(self.columns).enumerate() {
            line.push_str("  ");
            line.push_str(&padded(value, self.column_widths[index]));
        }
    }
}

fn row_values(row: &Row) -> [&str; 4] {
    [&row.base, &row.changes, &row.divergence, &row.path]
}

fn measured_width<'a>(rows: &'a [&Row], header: &str, value: impl Fn(&'a Row) -> &'a str) -> usize {
    rows.iter()
        .copied()
        .map(value)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
        .max(UnicodeWidthStr::width(header))
}

pub(crate) fn pick(choices: Vec<Row>) -> Result<Option<Row>> {
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
    let mut mode = TerminalMode::enter(output)?;
    let selection = select_raw(mode.output(), choices);
    mode.restore()?;
    selection
}

fn select_raw(output: &mut impl Write, choices: &[Row]) -> Result<Option<Row>> {
    let mut selected = 0;
    let mut rendered_lines = render_picker(output, choices, selected)?;
    output.flush()?;
    loop {
        let key = match event::read().context("read picker input")? {
            Event::Key(key) => key,
            Event::Resize(_, _) => {
                redraw_picker(output, choices, selected, &mut rendered_lines)?;
                continue;
            }
            _ => continue,
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
            selected = next;
            redraw_picker(output, choices, selected, &mut rendered_lines)?;
        }
    }
}

fn render_picker(output: &mut impl Write, rows: &[Row], selected: usize) -> Result<usize> {
    let (columns, height) = terminal::size().unwrap_or((80, 24));
    if height < 3 {
        bail!("interactive Change selection requires at least 3 terminal rows");
    }
    let capacity = usize::from(height - 2);
    let start = selected.saturating_sub(capacity.saturating_sub(1));
    let visible = &rows[start..rows.len().min(start + capacity)];
    let rows = visible.iter().collect::<Vec<_>>();
    let layout = TableLayout::new(&rows, usize::from(columns.saturating_sub(1)), 2);
    writeln!(output, "{}\r", bold(&layout.header(), navigator_styling()))?;
    for (index, row) in rows.into_iter().enumerate() {
        writeln!(output, "{}\r", layout.row(row, start + index == selected))?;
    }
    Ok(visible.len() + 1)
}

fn redraw_picker(
    output: &mut impl Write,
    rows: &[Row],
    selected: usize,
    rendered_lines: &mut usize,
) -> Result<()> {
    clear_rendered(output, *rendered_lines)?;
    *rendered_lines = 0;
    *rendered_lines = render_picker(output, rows, selected)?;
    output.flush()?;
    Ok(())
}

struct TerminalMode<'a, W: Write> {
    output: &'a mut W,
    active: bool,
}

impl<'a, W: Write> TerminalMode<'a, W> {
    fn enter(output: &'a mut W) -> Result<Self> {
        enable_raw_mode().context("enable raw mode for terminal selection")?;
        let mode = Self {
            output,
            active: true,
        };
        mode.output
            .queue(Hide)
            .and_then(|output| output.flush())
            .context("open terminal selection")?;
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
        self.output
            .queue(Show)?
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

pub(crate) fn change_rows(git: &Git) -> Result<Vec<Row>> {
    let worktrees = git.inventory()?;
    let current = git.current_path()?;
    let labels = title_labels(
        worktrees
            .iter()
            .map(|worktree| (worktree.id.as_str(), worktree.title.as_deref())),
        &["Main"],
    );
    let mut rows = Vec::new();
    for (worktree, title_label) in worktrees.iter().zip(labels) {
        let (available, changes) = match &worktree.state {
            WorktreeState::Missing => (false, "missing".to_owned()),
            WorktreeState::Present(status) => (true, format_changes(status)),
        };
        rows.push(Row {
            current: worktree.current,
            available,
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
    Ok(rows)
}

fn main_row(git: &Git) -> Result<Row> {
    let current = git.current_path()?;
    let primary = git.primary_path()?;
    Ok(Row {
        current: current == primary,
        available: true,
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
pub(crate) struct Row {
    current: bool,
    available: bool,
    change_id: Option<String>,
    worktree_path: PathBuf,
    title_label: String,
    base: String,
    changes: String,
    divergence: String,
    path: String,
}

impl Row {
    fn require_available(&self) -> Result<()> {
        if !self.available || !self.worktree_path.is_dir() {
            bail!(
                "Change workspace is missing: {}",
                self.worktree_path.display()
            );
        }
        Ok(())
    }

    pub(crate) fn is_current(&self) -> bool {
        self.current
    }

    pub(crate) fn workspace(&self) -> &Path {
        &self.worktree_path
    }

    pub(crate) fn change_id(&self) -> Option<&str> {
        self.change_id.as_deref()
    }

    pub(crate) fn title_label(&self) -> &str {
        &self.title_label
    }
}

fn format_changes(status: &git::Status) -> String {
    let mut parts = Vec::new();
    if status.added > 0 {
        parts.push(format!("+{}", status.added));
    }
    if status.deleted > 0 {
        parts.push(format!("-{}", status.deleted));
    }
    if status.untracked > 0 {
        parts.push(format!("?{}", status.untracked));
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

fn padded(value: &str, width: usize) -> String {
    let value = value
        .chars()
        .filter(|character| display_safe(*character))
        .collect::<String>();
    format!(
        "{value}{}",
        " ".repeat(width.saturating_sub(UnicodeWidthStr::width(value.as_str())))
    )
}

pub(crate) fn fit_width(mut value: String, max_width: Option<usize>) -> String {
    value.retain(display_safe);
    let Some(max_width) = max_width else {
        return value;
    };
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
