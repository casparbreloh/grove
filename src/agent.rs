use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, de::DeserializeOwned};

use crate::{
    git::{Git, PendingChange},
    runtime,
};

const DEFAULT_AGENT: &str = "pi";
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const NAMING_GRACE: Duration = Duration::from_millis(500);

pub(crate) struct Agent {
    name: String,
    builtin: Option<BuiltinAgent>,
    command: Vec<String>,
}

#[derive(Clone, Copy)]
enum BuiltinAgent {
    Pi,
    Claude,
    Codex,
}

impl BuiltinAgent {
    fn named(name: &str) -> Option<Self> {
        match name {
            "pi" => Some(Self::Pi),
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    fn command(self) -> Vec<String> {
        let executable = match self {
            Self::Pi => "pi",
            Self::Claude => "claude",
            Self::Codex => "codex",
        };
        vec![executable.to_owned()]
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalConfig {
    agent: Option<String>,
    #[serde(default)]
    agents: HashMap<String, AgentConfig>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectConfig {
    agent: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentConfig {
    command: Vec<String>,
}

impl Agent {
    fn load(git: &Git) -> Result<Self> {
        let global_path = global_config_path();
        let global: GlobalConfig = global_path
            .as_deref()
            .map(read_config)
            .transpose()?
            .unwrap_or_default();
        let project: ProjectConfig = read_config(&git.project_root()?.join("grove.toml"))?;
        let name = project
            .agent
            .as_deref()
            .or(global.agent.as_deref())
            .unwrap_or(DEFAULT_AGENT);
        let builtin = BuiltinAgent::named(name);
        let command = global
            .agents
            .get(name)
            .map(|config| config.command.clone())
            .or_else(|| builtin.map(BuiltinAgent::command))
            .with_context(|| format!("agent '{name}' is not configured"))?;
        if command.is_empty() {
            bail!("agent '{name}' requires command arguments");
        }
        Ok(Self {
            name: name.to_owned(),
            builtin,
            command,
        })
    }

    pub(crate) fn attach(self, git: &Git) -> Result<()> {
        runtime::attach(&git.worktree_identity()?, self.command)
    }

    pub(crate) fn load_for_launch(git: &Git) -> Result<Self> {
        let agent = Self::load(git)?;
        agent.validate_launch(git)?;
        Ok(agent)
    }

    pub(crate) fn load_for_switch(git: &Git) -> Result<Self> {
        Self::load(git)
    }

    fn validate_launch(&self, git: &Git) -> Result<()> {
        if !executable_exists(&self.command[0], &git.project_root()?) {
            bail!(
                "agent executable '{}' was not found or is not executable",
                self.command[0]
            );
        }
        Ok(())
    }

    pub(crate) fn load_for_naming(git: &Git) -> Result<Self> {
        let agent = Self::load(git)?;
        if agent.builtin.is_none() {
            bail!(
                "automatic branch naming is not supported for agent '{}'",
                agent.name
            );
        }
        agent.validate_launch(git)?;
        Ok(agent)
    }

    pub(crate) fn attach_and_name(mut self, git: &Git, pending: PendingChange) -> Result<()> {
        let identity = git.worktree_identity()?;
        let source = match self.builtin {
            Some(BuiltinAgent::Pi) => {
                let session = pi_session_path(pending.path())?;
                let existing = jsonl_files(
                    session
                        .parent()
                        .context("Pi session has no parent directory")?,
                )?;
                self.command
                    .extend(["--session".to_owned(), session.display().to_string()]);
                PromptSource::Pi {
                    session,
                    cwd: identity.root.clone(),
                    existing,
                }
            }
            Some(BuiltinAgent::Claude) => {
                let session_id = session_id()?;
                self.command
                    .extend(["--session-id".to_owned(), session_id.clone()]);
                PromptSource::Claude {
                    root: claude_projects_path()?,
                    session_id,
                }
            }
            Some(BuiltinAgent::Codex) => {
                let root = codex_sessions_path()?;
                let before = jsonl_files(&root)?;
                PromptSource::Codex {
                    root,
                    before,
                    cwd: git.worktree_identity()?.root,
                }
            }
            None => {
                bail!(
                    "automatic branch naming is not supported for agent '{}'",
                    self.name
                )
            }
        };
        let (cancel_sender, cancel_receiver) = mpsc::channel();
        let (outcome_sender, outcome_receiver) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let transcript = source.disposable_transcript();
            let outcome = match source.wait(&cancel_receiver) {
                Ok(Some(prompt)) => {
                    match branch_from_prompt(&prompt).and_then(|branch| pending.name(&branch)) {
                        Ok(_) => NamingOutcome::Named,
                        Err(error) => NamingOutcome::Failed(error.context(format!(
                            "pending worktree preserved at {}",
                            pending.path().display()
                        ))),
                    }
                }
                Ok(None) => NamingOutcome::Cancelled {
                    pending,
                    transcript,
                },
                Err(error) => NamingOutcome::Failed(error.context(format!(
                    "pending worktree preserved at {}",
                    pending.path().display()
                ))),
            };
            let _ = outcome_sender.send(outcome);
        });

        let attachment = runtime::attach(&identity, self.command);
        let wait = if attachment.is_ok() {
            NAMING_GRACE
        } else {
            Duration::ZERO
        };
        let outcome = match outcome_receiver.recv_timeout(wait) {
            Ok(outcome) => outcome,
            Err(RecvTimeoutError::Timeout) => {
                let _ = cancel_sender.send(());
                outcome_receiver
                    .recv()
                    .context("branch naming worker stopped unexpectedly")?
            }
            Err(RecvTimeoutError::Disconnected) => {
                bail!("branch naming worker stopped unexpectedly")
            }
        };

        match outcome {
            NamingOutcome::Named => attachment,
            NamingOutcome::Failed(error) => Err(error).context("could not infer branch"),
            NamingOutcome::Cancelled {
                pending,
                transcript,
            } => {
                if let Err(error) = pending.discard() {
                    attachment?;
                    return Err(error).context(format!(
                        "detached before the first prompt; pending worktree preserved at {}",
                        pending.path().display()
                    ));
                }
                runtime::terminate(&identity)?;
                if let Some(transcript) = transcript {
                    remove_transcript(&transcript)?;
                }
                attachment?;
                bail!("detached before the first prompt; removed the pending worktree")
            }
        }
    }
}

fn executable_exists(command: &str, cwd: &Path) -> bool {
    let command = Path::new(command);
    if command.components().count() > 1 {
        let path = if command.is_absolute() {
            command.to_owned()
        } else {
            cwd.join(command)
        };
        return is_executable(&path);
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|directory| is_executable(&directory.join(command)))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    return metadata.is_file() && metadata.permissions().mode() & 0o111 != 0;
    #[cfg(not(unix))]
    return metadata.is_file();
}

enum NamingOutcome {
    Named,
    Failed(anyhow::Error),
    Cancelled {
        pending: PendingChange,
        transcript: Option<PathBuf>,
    },
}

enum PromptSource {
    Pi {
        session: PathBuf,
        cwd: PathBuf,
        existing: HashSet<PathBuf>,
    },
    Claude {
        root: PathBuf,
        session_id: String,
    },
    Codex {
        root: PathBuf,
        before: HashSet<PathBuf>,
        cwd: PathBuf,
    },
}

impl PromptSource {
    fn disposable_transcript(&self) -> Option<PathBuf> {
        match self {
            Self::Pi { session, .. } => Some(session.clone()),
            Self::Claude { .. } | Self::Codex { .. } => None,
        }
    }

    fn wait(self, cancel: &Receiver<()>) -> Result<Option<String>> {
        match self {
            Self::Pi {
                session,
                cwd,
                existing,
            } => wait_for_pi_prompt(&session, &cwd, &existing, cancel),
            Self::Claude { root, session_id } => wait_for_claude_prompt(&root, &session_id, cancel),
            Self::Codex { root, before, cwd } => {
                wait_for_codex_prompt(&root, &before, &cwd, cancel)
            }
        }
    }
}

fn remove_transcript(path: &Path) -> Result<()> {
    fs::remove_file(path)
        .with_context(|| format!("failed to remove unused transcript {}", path.display()))?;
    if let Some(directory) = path.parent() {
        match fs::remove_dir(directory) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to clean up {}", directory.display()));
            }
        }
    }
    Ok(())
}

fn pi_session_path(worktree: &Path) -> Result<PathBuf> {
    let directory = worktree
        .parent()
        .context("pending worktree has no parent")?
        .join(".sessions");
    fs::create_dir_all(&directory)
        .with_context(|| format!("failed to create {}", directory.display()))?;
    let name = worktree
        .file_name()
        .context("pending worktree has no directory name")?;
    let path = directory.join(name).with_extension("pi.jsonl");
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    options
        .open(&path)
        .with_context(|| format!("failed to create Pi session {}", path.display()))?;
    Ok(path)
}

fn wait_for_pi_prompt(
    requested: &Path,
    cwd: &Path,
    existing: &HashSet<PathBuf>,
    cancel: &Receiver<()>,
) -> Result<Option<String>> {
    let directory = requested.parent().context("Pi session has no parent")?;
    loop {
        let mut candidates = vec![requested.to_owned()];
        candidates.extend(jsonl_files(directory)?.difference(existing).cloned());
        for candidate in candidates {
            if let Some(prompt) = pi_prompt_in_file(&candidate, requested, cwd)? {
                return Ok(Some(prompt));
            }
        }
        if cancelled(cancel) {
            return Ok(None);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn pi_prompt_in_file(path: &Path, requested: &Path, cwd: &Path) -> Result<Option<String>> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read transcript {}", path.display()));
        }
    };
    let mut matches_worktree = path == requested;
    let mut first_prompt = None;
    for line in contents.split(|byte| *byte == b'\n') {
        let Ok(entry) = serde_json::from_slice::<serde_json::Value>(line) else {
            continue;
        };
        if entry.get("type").and_then(|value| value.as_str()) == Some("session")
            && entry
                .get("cwd")
                .and_then(|value| value.as_str())
                .map(Path::new)
                == Some(cwd)
        {
            matches_worktree = true;
        }
        if first_prompt.is_none()
            && entry.get("type").and_then(|value| value.as_str()) == Some("message")
            && entry
                .pointer("/message/role")
                .and_then(|value| value.as_str())
                == Some("user")
        {
            first_prompt = message_content(&entry["message"]["content"]);
        }
    }
    Ok(matches_worktree.then_some(first_prompt).flatten())
}

fn claude_projects_path() -> Result<PathBuf> {
    let root = std::env::var_os("CLAUDE_CONFIG_DIR")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude")))
        .context("HOME is not set")?;
    Ok(root.join("projects"))
}

fn session_id() -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_nanos();
    let digest = blake3::hash(format!("{now}:{}", std::process::id()).as_bytes()).to_hex();
    Ok(format!(
        "{}-{}-4{}-8{}-{}",
        &digest[..8],
        &digest[8..12],
        &digest[13..16],
        &digest[17..20],
        &digest[20..32]
    ))
}

fn wait_for_claude_prompt(
    root: &Path,
    session_id: &str,
    cancel: &Receiver<()>,
) -> Result<Option<String>> {
    let filename = format!("{session_id}.jsonl");
    loop {
        if cancelled(cancel) {
            return Ok(None);
        }
        if let Some(path) = find_file(root, &filename)? {
            return wait_for_jsonl_prompt(&path, cancel, |entry| {
                if entry.get("type").and_then(|value| value.as_str()) != Some("user")
                    || entry
                        .pointer("/origin/kind")
                        .and_then(|value| value.as_str())
                        != Some("human")
                    || entry.get("promptSource").and_then(|value| value.as_str()) != Some("typed")
                {
                    return None;
                }
                message_content(&entry["message"]["content"])
            });
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_jsonl_prompt(
    path: &Path,
    cancel: &Receiver<()>,
    mut prompt: impl FnMut(&serde_json::Value) -> Option<String>,
) -> Result<Option<String>> {
    let file = fs::File::open(path)
        .with_context(|| format!("failed to read transcript {}", path.display()))?;
    let mut reader = BufReader::new(file);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            if cancelled(cancel) {
                return Ok(None);
            }
            thread::sleep(POLL_INTERVAL);
            continue;
        }
        let Ok(entry) = serde_json::from_str(&line) else {
            continue;
        };
        if let Some(prompt) = prompt(&entry) {
            return Ok(Some(prompt));
        }
    }
}

fn find_file(root: &Path, filename: &str) -> Result<Option<PathBuf>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", root.display()));
        }
    };
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, filename)? {
                return Ok(Some(found));
            }
        } else if path.file_name().is_some_and(|name| name == filename) {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn codex_sessions_path() -> Result<PathBuf> {
    let root = std::env::var_os("CODEX_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .context("HOME is not set")?;
    Ok(root.join("sessions"))
}

fn wait_for_codex_prompt(
    root: &Path,
    before: &HashSet<PathBuf>,
    cwd: &Path,
    cancel: &Receiver<()>,
) -> Result<Option<String>> {
    loop {
        if cancelled(cancel) {
            return Ok(None);
        }
        let mut matching = Vec::new();
        for path in jsonl_files(root)?.difference(before) {
            if let Some(prompt) = codex_prompt(path, cwd)? {
                matching.push(prompt);
            }
        }
        match matching.len() {
            0 => thread::sleep(POLL_INTERVAL),
            1 => {
                return matching
                    .pop()
                    .map(Some)
                    .context("matching Codex prompt disappeared");
            }
            _ => bail!("multiple new Codex sessions match the pending worktree"),
        }
    }
}

fn cancelled(cancel: &Receiver<()>) -> bool {
    match cancel.try_recv() {
        Ok(()) | Err(mpsc::TryRecvError::Disconnected) => true,
        Err(mpsc::TryRecvError::Empty) => false,
    }
}

fn codex_prompt(path: &Path, cwd: &Path) -> Result<Option<String>> {
    let file = fs::File::open(path)
        .with_context(|| format!("failed to read Codex rollout {}", path.display()))?;
    let mut matches_cwd = false;
    let mut prompt = None;
    for line in BufReader::new(file).lines() {
        let line = line?;
        let entry: serde_json::Value = match serde_json::from_str(&line) {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        match entry.get("type").and_then(|value| value.as_str()) {
            Some("session_meta") => {
                matches_cwd = entry
                    .pointer("/payload/cwd")
                    .and_then(|value| value.as_str())
                    .is_some_and(|recorded| same_path(Path::new(recorded), cwd));
            }
            Some("response_item")
                if entry
                    .pointer("/payload/type")
                    .and_then(|value| value.as_str())
                    == Some("message")
                    && entry
                        .pointer("/payload/role")
                        .and_then(|value| value.as_str())
                        == Some("user")
                    && prompt.is_none() =>
            {
                prompt = input_text_content(&entry["payload"]["content"]);
            }
            _ => {}
        }
    }
    Ok(matches_cwd.then_some(prompt).flatten())
}

fn same_path(left: &Path, right: &Path) -> bool {
    left.canonicalize().unwrap_or_else(|_| left.to_owned())
        == right.canonicalize().unwrap_or_else(|_| right.to_owned())
}

fn jsonl_files(root: &Path) -> Result<HashSet<PathBuf>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashSet::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", root.display()));
        }
    };
    let mut files = HashSet::new();
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(jsonl_files(&path)?);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "jsonl")
        {
            files.insert(path);
        }
    }
    Ok(files)
}

fn message_content(value: &serde_json::Value) -> Option<String> {
    if let Some(content) = value.as_str() {
        return Some(content.to_owned());
    }
    let content = value
        .as_array()?
        .iter()
        .filter(|part| part.get("type").and_then(|value| value.as_str()) == Some("text"))
        .filter_map(|part| part.get("text").and_then(|value| value.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    (!content.is_empty()).then_some(content)
}

fn input_text_content(value: &serde_json::Value) -> Option<String> {
    let content = value
        .as_array()?
        .iter()
        .filter(|part| {
            matches!(
                part.get("type").and_then(|value| value.as_str()),
                Some("input_text" | "text")
            )
        })
        .filter_map(|part| part.get("text").and_then(|value| value.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    (!content.is_empty()).then_some(content)
}

fn branch_from_prompt(prompt: &str) -> Result<String> {
    let mut branch = String::new();
    let mut separator = false;
    for character in prompt.trim().chars() {
        if character.is_ascii_alphanumeric() {
            let needs_separator = separator && !branch.is_empty();
            if branch.len() + usize::from(needs_separator) + 1 > 48 {
                break;
            }
            if needs_separator {
                branch.push('-');
            }
            branch.push(character.to_ascii_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }
    while branch.ends_with('-') {
        branch.pop();
    }
    if branch.is_empty() {
        bail!("the first prompt does not contain a branch name");
    }
    Ok(branch)
}

fn read_config<T: Default + DeserializeOwned>(path: &std::path::Path) -> Result<T> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))
}

fn global_config_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|root| root.join("grove/grove.toml"))
}
