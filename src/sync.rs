use std::{
    collections::HashMap,
    io::{IsTerminal, Write},
};

use anyhow::Result;
use crossterm::terminal;
use unicode_width::UnicodeWidthStr;

use crate::{git::Git, navigator::fit_width};

pub(crate) fn run(git: &Git) -> Result<()> {
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
