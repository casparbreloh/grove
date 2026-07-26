mod archive;
mod change;
mod doctor;
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
struct GroveCli {
    #[arg(long, hide = true)]
    usage_spec: bool,

    #[command(subcommand)]
    command: Option<GroveCommand>,
}

#[derive(Subcommand)]
enum GroveCommand {
    /// Create a Change workspace and open Pi
    ///
    /// Managed Pi makes an additional, asynchronous provider request from the
    /// first prompt to infer a title.
    New {
        /// Start the Change from this revision (`@` means the invoking worktree)
        #[arg(long, value_name = "REF")]
        from: Option<String>,
    },
    /// Update Main, clean merged branches, and synchronize every Change
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
    /// Diagnose local Grove and Git state without changing it
    Doctor,
    /// Print shell integration and completions
    Init { shell: ShellKind },
    #[command(name = "__title", hide = true)]
    Title {
        #[arg(long)]
        change: String,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Clone, ValueEnum)]
enum ShellKind {
    Fish,
    Zsh,
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {}", change::display_text(&format!("{error:#}")));
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    clap_complete::CompleteEnv::with_factory(GroveCli::command).complete();

    let cli = GroveCli::parse();
    if cli.usage_spec {
        clap_usage::generate(&mut GroveCli::command(), "grove", &mut std::io::stdout());
        return Ok(());
    }

    match cli.command {
        None => navigator::run_navigator(&Git::discover()?),
        Some(GroveCommand::New { from }) => new::run_new(&Git::discover()?, from.as_deref()),
        Some(GroveCommand::Sync) => sync::run_sync(&Git::discover()?),
        Some(GroveCommand::Ship) => ship::run_ship(&Git::discover()?),
        Some(GroveCommand::Archive { force }) => archive::run_archive(&Git::discover()?, force),
        Some(GroveCommand::Doctor) => doctor::run_doctor(&Git::discover()?),
        Some(GroveCommand::Init { shell }) => init::run_init(shell),
        Some(GroveCommand::Title { change, apply }) => {
            if apply {
                session::apply_change_title(&change)
            } else {
                session::name_change(&change)
            }
        }
    }
}
