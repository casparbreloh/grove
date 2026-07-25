use std::io::Write;

use anyhow::Result;
use clap_complete::env::{EnvCompleter, Fish as FishCompleter, Zsh as ZshCompleter};

use crate::Shell;

pub(crate) fn run(shell: Shell) -> Result<()> {
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
