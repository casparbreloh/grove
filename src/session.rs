use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

use anyhow::{Context, Result, bail};

use crate::change;

const EXTENSION: &[u8] = include_bytes!("pi-extension.ts");
const TITLE_SYSTEM_PROMPT: &str = "Create a concise title of exactly three or four words for the user's request. Output only the title on one line, with no quotes, punctuation-only words, explanation, or prefix.";

pub(crate) struct Session {
    capsule: PathBuf,
    workspace: PathBuf,
}

impl Session {
    pub(crate) fn for_workspace(workspace: &Path) -> Result<Self> {
        let capsule = workspace
            .parent()
            .context("Grove workspace has no Change capsule")?
            .to_owned();
        Ok(Self {
            capsule,
            workspace: workspace.to_owned(),
        })
    }

    pub(crate) fn prepare() -> Result<()> {
        validate_pi()
    }

    pub(crate) fn attach(&self) -> Result<()> {
        validate_pi()?;
        let _lock = self.lock()?;
        let sessions = self.capsule.join("pi");
        create_private_directory_all(&sessions).with_context(|| {
            format!(
                "failed to create Pi session directory {}",
                sessions.display()
            )
        })?;
        let extension = materialize_extension()?;
        let executable = env::current_exe().context("failed to locate the Grove executable")?;
        let change_id = self
            .capsule
            .file_name()
            .and_then(|name| name.to_str())
            .context("change capsule has no valid ID")?;
        let status = Command::new("pi")
            .arg("--session-dir")
            .arg(&sessions)
            .arg("--continue")
            .arg("--extension")
            .arg(&extension)
            .current_dir(&self.workspace)
            .env("GROVE_EXECUTABLE", executable)
            .env("GROVE_CHANGE_ID", change_id)
            .env("GROVE_CHANGE_CAPSULE", &self.capsule)
            .env_remove("GROVE_DIRECTIVE_CD_FILE")
            .status();
        let _ = fs::remove_file(&extension);
        let status = status
            .with_context(|| format!("failed to launch Pi in {}", self.workspace.display()))?;
        if !status.success() {
            bail!("Pi exited with {status} in {}", self.workspace.display());
        }
        Ok(())
    }

    pub(crate) fn lock(&self) -> Result<change::Lock> {
        change::lock(&self.capsule)
    }
}

pub(crate) fn infer_title(
    capsule: &Path,
    change_id: &str,
    session_id: &str,
    prompt: &str,
) -> Result<String> {
    validate_pi()?;
    let session_bytes = session_id.as_bytes();
    if !session_bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        || !session_bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        || !session_bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!("invalid Pi session identity");
    }
    if prompt.trim().is_empty() {
        bail!("cannot infer a title from an empty prompt");
    }

    let output = Command::new("pi")
        .arg("--print")
        .arg("--no-session")
        .arg("--no-tools")
        .arg("--no-context-files")
        .arg("--no-skills")
        .arg("--no-extensions")
        .arg("--system-prompt")
        .arg(TITLE_SYSTEM_PROMPT)
        .arg(prompt)
        .current_dir(capsule.join("workspace"))
        .output()
        .with_context(|| "failed to launch isolated Pi title generator")?;
    if !output.status.success() {
        bail!("Pi title generator exited with {}", output.status);
    }
    let output = String::from_utf8(output.stdout).context("Pi title was not valid UTF-8")?;
    let title = output.trim();
    let words = title.split_whitespace().collect::<Vec<_>>();
    if title.is_empty()
        || title.len() > 80
        || title.contains(['\r', '\n'])
        || !(3..=4).contains(&words.len())
        || words
            .iter()
            .any(|word| !word.chars().any(char::is_alphanumeric))
    {
        bail!("Pi returned an invalid title");
    }

    change::initialize_title(capsule, change_id, title)?;
    Ok(title.to_owned())
}

fn materialize_extension() -> Result<PathBuf> {
    let temporary = temporary_path(&env::temp_dir(), "grove-pi-extension");
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .with_context(|| format!("failed to create {}", temporary.display()))?;
    file.write_all(EXTENSION)
        .with_context(|| format!("failed to write {}", temporary.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync {}", temporary.display()))?;
    Ok(temporary)
}

fn temporary_path(parent: &Path, label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    parent.join(format!(".{label}-{}-{nonce}", std::process::id()))
}

fn create_private_directory_all(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(path)
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
