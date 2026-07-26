use std::io::Write;

use anyhow::{Result, bail};

use crate::{
    change::title_labels,
    git::{Git, SyncAction},
};

pub(crate) fn run_sync(git: &Git) -> Result<()> {
    let entries = git.sync()?;
    let labels = title_labels(
        entries
            .iter()
            .map(|entry| (entry.id.as_str(), entry.title.as_deref())),
        &[],
    );
    let mut rows = entries
        .iter()
        .zip(labels)
        .filter_map(|(entry, label)| {
            let action = entry.action?;
            let order = match action {
                SyncAction::Archived => 0,
                SyncAction::Rebased => 1,
                SyncAction::ConflictRestored => 2,
            };
            Some((order, label, action))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    let conflicts = rows
        .iter()
        .filter(|(_, _, action)| matches!(action, SyncAction::ConflictRestored))
        .count();
    let mut output = std::io::stderr().lock();
    for (_, label, action) in rows {
        match action {
            SyncAction::Archived => writeln!(output, "✓ Archived {label}")?,
            SyncAction::Rebased => writeln!(output, "✓ Rebased {label}")?,
            SyncAction::ConflictRestored => {
                writeln!(output, "! Could not rebase {label}; restored unchanged")?
            }
        }
    }
    output.flush()?;
    drop(output);

    if conflicts > 0 {
        let suffix = if conflicts == 1 { "" } else { "s" };
        bail!("sync encountered {conflicts} rebase conflict{suffix}");
    }
    Ok(())
}
