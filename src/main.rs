mod archive;
mod change;
mod git;
mod init;
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
        /// Start the Change from this revision (`@` means the invoking worktree)
        #[arg(long, value_name = "REF")]
        from: Option<String>,
    },
    /// Update the primary worktree or rebase the current Change
    Sync,
    /// Ship the current Change as a pull request
    ///
    /// Uses an isolated provider request for shipping metadata, then commits,
    /// pushes, and creates or updates the pull request.
    Ship,
    /// Archive an active Change
    Archive {
        /// Irreversibly discard uncommitted and unintegrated work
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
        apply: bool,
    },
}

#[derive(Clone, ValueEnum)]
enum Shell {
    Fish,
    Zsh,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {}", change::display_text(&format!("{error:#}")));
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    if cli.usage_spec {
        clap_usage::generate(&mut Cli::command(), "grove", &mut std::io::stdout());
        return Ok(());
    }

    match cli.command {
        None => navigator::run(&Git::discover()?),
        Some(Cmd::New { from }) => new::run(&Git::discover()?, from.as_deref()),
        Some(Cmd::Sync) => sync::run(&Git::discover()?),
        Some(Cmd::Ship) => ship::run(&Git::discover()?),
        Some(Cmd::Archive { force }) => archive::run(&Git::discover()?, force),
        Some(Cmd::Init { shell }) => init::run(shell),
        Some(Cmd::Title { change, apply }) => {
            if apply {
                session::apply_change_title(&change)
            } else {
                session::name_change(&change)
            }
        }
    }
}
