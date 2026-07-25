use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::git::Git;

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
