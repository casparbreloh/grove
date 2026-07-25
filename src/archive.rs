use anyhow::{Context, Result, bail};

use crate::{
    git::Git,
    navigation,
    navigator::{change_rows, pick},
    session::Session,
};

pub(crate) fn run(git: &Git, force: bool) -> Result<()> {
    let recovered = git.recover_closing_archives()?;
    if recovered > 0 {
        eprintln!("✓ Finished {recovered} interrupted archive(s)");
        return Ok(());
    }
    let current = git.current_path()?;
    let rows = change_rows(git)?;
    let selected = if let Some(current) = rows.iter().find(|row| row.is_current()) {
        Some(current.clone())
    } else if current == git.primary_path()? {
        pick(rows)?
    } else {
        bail!("current workspace is not a managed Grove Change");
    };
    let Some(selected) = selected else {
        return Ok(());
    };
    let archive_destination = if selected.workspace() == current {
        navigation::require_shell_navigation()?;
        Some(navigation::destination(git, &git.primary_path()?)?)
    } else {
        None
    };
    let session = Session::for_workspace(selected.workspace())?;
    let _lock = session.lock()?;
    let change_id = selected
        .change_id()
        .context("selected destination is not a Change")?;
    let prepared = git.prepare_archive(change_id, force)?;
    git.finish_archive(prepared)?;
    eprintln!("✓ Archived {}", selected.title_label());
    if let Some(path) = archive_destination {
        navigation::navigate(&path)?;
    }
    Ok(())
}
