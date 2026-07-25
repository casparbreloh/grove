mod archive;
mod change;
mod git;
mod hosting;
mod init;
mod navigation;
mod navigator;
mod new;
mod session;
mod ship;
mod sync;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

use crate::git::Git;

#[derive(Parser)]
struct Cli {
    #[arg(long, hide = true)]
    usage_spec: bool,

    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a Change workspace and open Pi
    ///
    /// Managed Pi makes an additional, asynchronous provider request from the
    /// first prompt to infer a title.
    New {
        /// Start the change from this revision (`@` means the invoking worktree)
        #[arg(long, value_name = "REF")]
        from: Option<String>,
    },
    /// Fetch upstream, archive integrated Changes, and rebase eligible Changes
    Sync,
    /// Ship the current Change as a pull request with Pi
    Ship,
    /// Archive an active Change
    Archive {
        /// Archive and discard unmerged work
        #[arg(long)]
        force: bool,
    },
    /// Print shell integration and completions
    Init { shell: Shell },
    #[command(name = "__title", hide = true)]
    Title {
        #[arg(long)]
        change: String,
        #[arg(long)]
        session: String,
    },
}

#[derive(Clone, ValueEnum)]
enum Shell {
    Fish,
    Zsh,
}

fn main() -> Result<()> {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();

    if std::env::args_os().len() == 2
        && std::env::args_os().nth(1).as_deref() == Some("--usage-spec".as_ref())
    {
        clap_usage::generate(&mut Cli::command(), "grove", &mut std::io::stdout());
        return Ok(());
    }

    match Cli::parse().command {
        None => navigator::run(&Git::discover()?),
        Some(Cmd::New { from }) => new::run(&Git::discover()?, from.as_deref()),
        Some(Cmd::Sync) => sync::run(&Git::discover()?),
        Some(Cmd::Ship) => ship::run(&Git::discover()?),
        Some(Cmd::Archive { force }) => archive::run(&Git::discover()?, force),
        Some(Cmd::Init { shell }) => init::run(shell),
        Some(Cmd::Title { change, session }) => session::title(&change, &session),
    }
}
