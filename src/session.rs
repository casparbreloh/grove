use std::{
    env, fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;

use crate::git::{Git, PendingChange, WorktreeIdentity};

const ZMX_VERSION: &str = "0.6.0";
const EXTENSION: &[u8] = include_bytes!("pi-extension.ts");

pub(crate) enum PendingOutcome {
    Complete,
    Preserved,
}

pub(crate) fn start_pending(pending: &PendingChange) -> Result<PendingOutcome> {
    open_pending(pending, true)
}

pub(crate) fn resume_pending(pending: &PendingChange) -> Result<PendingOutcome> {
    open_pending(pending, false)
}

fn open_pending(pending: &PendingChange, rollback_launch: bool) -> Result<PendingOutcome> {
    let path = pending.path();
    let session = Session::for_launch(&Git::at(path)?)?;
    if let Err(attachment_error) = session.attach_pending() {
        if !rollback_launch {
            return Err(attachment_error).context(format!(
                "could not reopen pending worktree at {}",
                path.display()
            ));
        }
        if let Err(discard_error) = pending.discard() {
            return Err(discard_error).context(format!(
                "Pi could not be opened; pending worktree preserved at {}",
                path.display()
            ));
        }
        session.discard_unstarted()?;
        return Err(attachment_error);
    }
    if !pending.is_pending() || session.naming_started() {
        return Ok(PendingOutcome::Complete);
    }
    if session.active()? {
        return Ok(PendingOutcome::Preserved);
    }
    if let Err(error) = pending.discard() {
        return Err(error).context(format!(
            "Pi exited before the first prompt; pending worktree preserved at {}",
            path.display()
        ));
    }
    session.discard_unstarted()?;
    bail!("Pi exited before the first prompt; removed the pending worktree")
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const ZMX_ARCHIVE: &[u8] = include_bytes!("../vendor/zmx/zmx-0.6.0-macos-aarch64.tar.gz");
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const ZMX_ARCHIVE: &[u8] = include_bytes!("../vendor/zmx/zmx-0.6.0-macos-x86_64.tar.gz");
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const ZMX_ARCHIVE: &[u8] = include_bytes!("../vendor/zmx/zmx-0.6.0-linux-aarch64.tar.gz");
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const ZMX_ARCHIVE: &[u8] = include_bytes!("../vendor/zmx/zmx-0.6.0-linux-x86_64.tar.gz");
#[cfg(not(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "macos", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "x86_64")
)))]
compile_error!("Grove supports macOS and Linux on aarch64 and x86_64");

pub(crate) struct Session {
    name: Option<String>,
    git_dir: PathBuf,
    root: PathBuf,
}

impl Session {
    pub(crate) fn for_launch(git: &Git) -> Result<Self> {
        let identity = git.worktree_identity()?;
        if identity.session_id.is_some() {
            return Ok(Self::for_worktree(&identity));
        }
        validate_pi()?;
        Ok(Self::for_worktree(&git.session_identity()?))
    }

    pub(crate) fn for_worktree(identity: &WorktreeIdentity) -> Self {
        Self {
            name: identity.session_id.as_ref().map(|id| {
                let digest = blake3::hash(id.as_bytes()).to_hex();
                format!("g-{}", &digest[..16])
            }),
            git_dir: identity.git_dir.clone(),
            root: identity.root.clone(),
        }
    }

    pub(crate) fn prepare() -> Result<()> {
        validate_pi()?;
        zmx_path().map(|_| ())?;
        extension_path().map(|_| ())
    }

    pub(crate) fn attach(&self) -> Result<()> {
        self.open(false)
    }

    fn attach_pending(&self) -> Result<()> {
        self.open(true)
    }

    fn open(&self, name_pending: bool) -> Result<()> {
        let name = self.name()?;
        if !executable_exists("pi") && !self.active()? {
            bail!("Pi executable 'pi' was not found or is not executable");
        }
        fs::create_dir_all(runtime_dir()?).context("failed to create the ZMX runtime directory")?;
        let pi_session = self.pi_session_path()?;
        fs::create_dir_all(
            pi_session
                .parent()
                .context("Pi session path has no parent")?,
        )?;
        let mut command = zmx_command()?;
        command
            .args(["attach", name, "pi", "--session"])
            .arg(&pi_session)
            .arg("--extension")
            .arg(extension_path()?)
            .current_dir(&self.root)
            .env_remove("GROVE_DIRECTIVE_CD_FILE");
        if name_pending {
            let claim = self.naming_claim();
            command
                .env("GROVE_EXECUTABLE", std::env::current_exe()?)
                .env("GROVE_NAMING_CLAIM", claim);
        } else {
            command
                .env_remove("GROVE_EXECUTABLE")
                .env_remove("GROVE_NAMING_CLAIM");
        }
        let status = command.status().context("failed to open the Pi session")?;
        if !status.success() {
            bail!("Pi session exited with {status}");
        }
        Ok(())
    }

    pub(crate) fn active(&self) -> Result<bool> {
        let Some(name) = self.name.as_deref() else {
            return Ok(false);
        };
        if !runtime_dir()?.exists() {
            return Ok(false);
        }
        let output = zmx_command()?
            .args(["list", "--short"])
            .output()
            .context("failed to inspect Pi sessions")?;
        if !output.status.success() {
            bail!(
                "could not inspect Pi sessions: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line == name))
    }

    pub(crate) fn terminate(&self) -> Result<()> {
        let Some(name) = self.name.as_deref() else {
            return Ok(());
        };
        if !runtime_dir()?.exists() {
            return Ok(());
        }
        let output = zmx_command()?
            .args(["kill", name])
            .stdin(Stdio::null())
            .output()
            .context("failed to stop the Pi session")?;
        if output.status.success() || !self.active()? {
            return Ok(());
        }
        bail!(
            "could not stop Pi session {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }

    fn discard_unstarted(&self) -> Result<()> {
        self.terminate()?;
        let path = self.pi_session_path()?;
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("failed to remove {}", path.display()));
            }
        }
        Ok(())
    }

    fn naming_started(&self) -> bool {
        self.naming_claim().is_file()
    }

    fn name(&self) -> Result<&str> {
        self.name
            .as_deref()
            .context("worktree has no Grove session identity")
    }

    fn naming_claim(&self) -> PathBuf {
        self.git_dir.join("grove-naming-started")
    }

    fn pi_session_path(&self) -> Result<PathBuf> {
        Ok(state_dir()?
            .join("sessions")
            .join(format!("{}.jsonl", self.name()?)))
    }
}

fn zmx_path() -> Result<PathBuf> {
    let path = cache_dir()?.join(format!("zmx-{ZMX_VERSION}"));
    if path.is_file() {
        return Ok(path);
    }
    let parent = path.parent().context("ZMX cache path has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = temporary_path(parent, "zmx");
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o700);
    let mut file = options
        .open(&temporary)
        .with_context(|| format!("failed to create {}", temporary.display()))?;
    let mut archive = tar::Archive::new(GzDecoder::new(Cursor::new(ZMX_ARCHIVE)));
    let mut found = false;
    for entry in archive
        .entries()
        .context("failed to read embedded ZMX archive")?
    {
        let mut entry = entry.context("failed to read embedded ZMX entry")?;
        if entry.path()?.as_ref() == Path::new("zmx") {
            std::io::copy(&mut entry, &mut file).context("failed to extract embedded ZMX")?;
            found = true;
            break;
        }
    }
    if !found {
        let _ = fs::remove_file(&temporary);
        bail!("embedded ZMX archive does not contain zmx");
    }
    file.sync_all()?;
    #[cfg(unix)]
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o700))?;
    install(&temporary, &path)?;
    Ok(path)
}

fn extension_path() -> Result<PathBuf> {
    let digest = blake3::hash(EXTENSION).to_hex();
    let path = cache_dir()?.join(format!("pi-extension-{}.ts", &digest[..12]));
    if path.is_file() {
        return Ok(path);
    }
    let parent = path
        .parent()
        .context("extension cache path has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = temporary_path(parent, "extension");
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&temporary)?;
    file.write_all(EXTENSION)?;
    file.sync_all()?;
    install(&temporary, &path)?;
    Ok(path)
}

fn install(temporary: &Path, target: &Path) -> Result<()> {
    match fs::rename(temporary, target) {
        Ok(()) => Ok(()),
        Err(_error) if target.is_file() => {
            let _ = fs::remove_file(temporary);
            Ok(())
        }
        Err(error) => Err(error).with_context(|| format!("failed to install {}", target.display())),
    }
}

fn temporary_path(parent: &Path, label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    parent.join(format!(".{label}-{}-{nonce}", std::process::id()))
}

fn cache_dir() -> Result<PathBuf> {
    Ok(data_root("XDG_CACHE_HOME", ".cache")?
        .join("grove")
        .join("runtime"))
}

fn state_dir() -> Result<PathBuf> {
    Ok(data_root("XDG_STATE_HOME", ".local/state")?.join("grove"))
}

fn runtime_dir() -> Result<PathBuf> {
    if let Some(base) = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("TMPDIR").filter(|value| !value.is_empty()))
    {
        return Ok(PathBuf::from(base).join("grove"));
    }
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    let digest = blake3::hash(Path::new(&home).as_os_str().as_encoded_bytes()).to_hex();
    Ok(PathBuf::from("/tmp").join(format!("grove-{}", &digest[..12])))
}

fn zmx_command() -> Result<Command> {
    let mut command = Command::new(zmx_path()?);
    command
        .env("ZMX_DIR", runtime_dir()?)
        .env_remove("ZMX_SESSION_PREFIX");
    Ok(command)
}

fn validate_pi() -> Result<()> {
    if !executable_exists("pi") {
        bail!("Pi executable 'pi' was not found or is not executable");
    }
    Ok(())
}

fn executable_exists(command: &str) -> bool {
    let command = Path::new(command);
    env::var_os("PATH")
        .map(|path| {
            env::split_paths(&path).any(|directory| {
                let path = directory.join(command);
                let Ok(metadata) = path.metadata() else {
                    return false;
                };
                #[cfg(unix)]
                return metadata.is_file() && metadata.permissions().mode() & 0o111 != 0;
                #[cfg(not(unix))]
                return metadata.is_file();
            })
        })
        .unwrap_or(false)
}

fn data_root(variable: &str, fallback: &str) -> Result<PathBuf> {
    if let Some(root) = std::env::var_os(variable).filter(|value| !value.is_empty()) {
        return Ok(root.into());
    }
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(fallback))
}
