use anyhow::{Context, Result, bail};

use crate::{
    change,
    git::Git,
    navigator::{
        build_change_rows, navigate_calling_shell, pick_change, require_shell_navigation,
        workspace_destination,
    },
};

pub(crate) fn run_archive(git: &Git, force: bool) -> Result<()> {
    let current = git.current_path()?;
    let primary = git.primary_path()?;
    if current != primary {
        require_shell_navigation()?;
        if !git.is_managed_change_path(&current)? {
            bail!("current workspace is not a managed Grove Change");
        }
    }
    let recovered = git.recover_closing_archives()?;
    if recovered > 0 {
        let suffix = if recovered == 1 { "" } else { "s" };
        eprintln!("✓ Recovered {recovered} interrupted archive{suffix}");
    }
    let rows = build_change_rows(git)?;
    let selected = if let Some(current) = rows.iter().find(|row| row.is_current()) {
        Some(current.clone())
    } else if current == primary {
        if recovered > 0 && rows.is_empty() {
            return Ok(());
        }
        pick_change(rows)?
    } else {
        bail!("current workspace is not a managed Grove Change");
    };
    let Some(selected) = selected else {
        return Ok(());
    };
    let archive_destination = if selected.workspace() == current {
        Some(workspace_destination(git, &primary)?)
    } else {
        None
    };
    let capsule = selected
        .workspace()
        .parent()
        .context("selected Change workspace has no capsule")?;
    let _lock = change::lock(capsule)?;
    let change_id = selected
        .change_id()
        .context("selected destination is not a Change")?;
    let prepared = git.prepare_archive(change_id, force)?;
    git.finish_archive(prepared)?;
    eprintln!("✓ Archived {}", selected.title_label());
    if let Some(path) = archive_destination {
        navigate_calling_shell(&path)?;
    }
    Ok(())
}
