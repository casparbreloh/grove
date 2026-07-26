use anyhow::{Context, Result, bail};

use crate::{
    change,
    git::Git,
    navigator::{change_rows, destination, navigate, pick, require_shell_navigation},
};

pub(crate) fn run(git: &Git, force: bool) -> Result<()> {
    let recovered = git.recover_closing_archives()?;
    if recovered > 0 {
        eprintln!("✓ Finished interrupted archives: {recovered}");
    }
    let current = git.current_path()?;
    let primary = git.primary_path()?;
    let rows = change_rows(git)?;
    let selected = if let Some(current) = rows.iter().find(|row| row.is_current()) {
        Some(current.clone())
    } else if current == primary {
        if recovered > 0 && rows.is_empty() {
            return Ok(());
        }
        pick(rows)?
    } else {
        bail!("current workspace is not a managed Grove Change");
    };
    let Some(selected) = selected else {
        return Ok(());
    };
    let archive_destination = if selected.workspace() == current {
        require_shell_navigation()?;
        Some(destination(git, &primary)?)
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
        navigate(&path)?;
    }
    Ok(())
}
