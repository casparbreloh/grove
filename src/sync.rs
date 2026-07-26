use std::io::{IsTerminal, Write};

use anyhow::Result;
use crossterm::terminal;
use unicode_width::UnicodeWidthStr;

use crate::{
    change::title_labels,
    git::{Git, SyncOutcome},
    navigator::fit_width,
};

pub(crate) fn run(git: &Git) -> Result<()> {
    let result = git.sync()?;
    let labels = title_labels(
        result
            .entries
            .iter()
            .map(|entry| (entry.id.as_str(), entry.title.as_deref())),
        &[],
    );
    let rows = result
        .entries
        .iter()
        .zip(labels)
        .map(|(entry, title)| {
            let (marker, outcome) = match entry.outcome {
                SyncOutcome::Archived => ('-', "archived"),
                SyncOutcome::Rebased => ('↑', "rebased"),
                SyncOutcome::Skipped => ('○', "skipped"),
            };
            (marker, title, outcome, entry.reason)
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
        result.count(SyncOutcome::Archived),
        result.count(SyncOutcome::Rebased),
        result.count(SyncOutcome::Skipped),
    )?;
    output.flush()?;
    Ok(())
}
