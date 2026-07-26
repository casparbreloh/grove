use std::io::Write;

use anyhow::{Context, Result};
use clap_complete::env::{EnvCompleter, Fish as FishCompleter, Zsh as ZshCompleter};

use crate::ShellKind;

pub(crate) fn run_init(shell: ShellKind) -> Result<()> {
    let executable = std::env::current_exe().context("failed to locate the Grove executable")?;
    let executable = executable.to_string_lossy();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    match shell {
        ShellKind::Fish => {
            FishCompleter.write_registration(
                "COMPLETE",
                "grove",
                "grove",
                &executable,
                &mut output,
            )?;
            output
                .write_all(include_bytes!("shell.fish"))
                .context("failed to write Fish integration")?;
        }
        ShellKind::Zsh => {
            ZshCompleter.write_registration(
                "COMPLETE",
                "grove",
                "grove",
                &executable,
                &mut output,
            )?;
            output
                .write_all(include_bytes!("shell.zsh"))
                .context("failed to write Zsh integration")?;
        }
    }
    Ok(())
}
