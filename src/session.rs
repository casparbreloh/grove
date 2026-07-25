use std::{
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::change;

const CHANGE_SESSION_EXTENSION: &[u8] = include_bytes!("extensions/change-session.ts");
const STRUCTURED_OUTPUT_EXTENSION: &[u8] = include_bytes!("extensions/structured-output.ts");
const WORKER_MODEL: &str = "openai-codex/gpt-5.6-sol";
const ACTIVITY_CAPABILITY: &str = "GROVE_ACTIVITY_CAPABILITY";
const CHANGE_PROMPT: &str = "Create a concise title of exactly three or four words for the user's request. Call structured_output with the title in the change field. Do not answer in any other way.";
const CHANGE_OUTPUT_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "change": { "type": "string", "minLength": 1, "maxLength": 80 }
  },
  "required": ["change"],
  "additionalProperties": false
}"#;
const SHIP_PROMPT: &str = "Write concise publication metadata for the supplied Change. Set commit to null when Published history is false because Grove uses the new pull-request title for the initial commit. Otherwise, a commit is a single Conventional Commit subject describing only newly staged work, with no body. Pull-request metadata describes the complete Change; return it only when a pull request must be created or its current title and body no longer fit. Treat the supplied summary as an index of the complete publication. Use it as primary context. If intent or scope is unclear, use read selectively until you understand the Change well enough to name it accurately. Avoid unrelated investigation. Finish only by calling structured_output; if tool calling is unavailable, return only JSON matching its schema.";
const SHIP_OUTPUT_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "commit": { "anyOf": [{ "type": "string" }, { "type": "null" }] },
    "pull_request": {
      "anyOf": [
        {
          "type": "object",
          "properties": {
            "title": { "type": "string" },
            "body": { "type": "string" }
          },
          "required": ["title", "body"],
          "additionalProperties": false
        },
        { "type": "null" }
      ]
    }
  },
  "required": ["commit", "pull_request"],
  "additionalProperties": false
}"#;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PullRequestMetadata {
    pub(crate) title: String,
    pub(crate) body: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ShipMetadata {
    pub(crate) commit: Option<String>,
    pub(crate) pull_request: Option<PullRequestMetadata>,
}

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
        let activity = change::managed_lock(&self.capsule)?;
        let sessions = self.capsule.join("pi");
        create_private_directory_all(&sessions).with_context(|| {
            format!(
                "failed to create Pi session directory {}",
                sessions.display()
            )
        })?;
        let executable = env::current_exe().context("failed to locate the Grove executable")?;
        let change_id = self
            .capsule
            .file_name()
            .and_then(|name| name.to_str())
            .context("change capsule has no valid ID")?;
        let extension = materialize_extension(CHANGE_SESSION_EXTENSION, "grove-session")?;
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
            .env(ACTIVITY_CAPABILITY, &activity.capability)
            .env_remove("GROVE_DIRECTIVE_CD_FILE")
            .status();
        drop(activity);
        let _ = fs::remove_file(&extension);
        let status = status
            .with_context(|| format!("failed to launch Pi in {}", self.workspace.display()))?;
        if !status.success() {
            bail!("Pi exited with {status} in {}", self.workspace.display());
        }
        Ok(())
    }

    pub(crate) fn generate_ship_metadata(&self, summary: &str) -> Result<ShipMetadata> {
        validate_pi()?;
        let value = run_structured_worker(
            &self.workspace,
            SHIP_PROMPT,
            SHIP_OUTPUT_SCHEMA,
            summary,
            "read,structured_output",
            "shipping metadata",
        )?;
        let metadata: ShipMetadata = serde_json::from_value(value)
            .context("Pi returned invalid structured shipping metadata")?;
        validate_ship_metadata(&metadata)?;
        Ok(metadata)
    }

    pub(crate) fn lock(&self) -> Result<change::Lock> {
        change::lock(&self.capsule)
    }

    pub(crate) fn lock_for_ship(&self) -> Result<change::ShipLock> {
        let capability = env::var(ACTIVITY_CAPABILITY).ok();
        if capability.is_some() {
            // Grove is single-threaded here and removes the bearer value before spawning workers.
            unsafe { env::remove_var(ACTIVITY_CAPABILITY) };
        }
        change::lock_for_ship(&self.capsule, capability.as_deref())
    }
}

pub(crate) fn name_change(change_id: &str, session_id: &str) -> Result<()> {
    let capsule = change_capsule()?;
    change::validate_identity(&capsule, change_id)?;
    validate_session_id(session_id)?;
    let mut prompt = String::new();
    std::io::stdin()
        .read_to_string(&mut prompt)
        .context("failed to read the title prompt")?;
    println!("{}", infer_change_title(&capsule, &prompt)?);
    Ok(())
}

pub(crate) fn apply_change_title(change_id: &str, session_id: &str) -> Result<()> {
    let capsule = change_capsule()?;
    validate_session_id(session_id)?;
    let capability = env::var(ACTIVITY_CAPABILITY)
        .context("Change title application is not owned by managed Pi")?;
    let _ownership = change::lock_for_managed_child(&capsule, &capability)?;
    let mut title = String::new();
    std::io::stdin()
        .read_to_string(&mut title)
        .context("failed to read the inferred title")?;
    let title = validate_change_title(&title)?;
    change::initialize_title(&capsule, change_id, title)
}

fn change_capsule() -> Result<PathBuf> {
    env::var_os("GROVE_CHANGE_CAPSULE")
        .map(PathBuf::from)
        .context("GROVE_CHANGE_CAPSULE is not set")
}

fn validate_session_id(session_id: &str) -> Result<()> {
    let session_bytes = session_id.as_bytes();
    if !session_bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        || !session_bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        || !session_bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!("invalid Pi session identity");
    }
    Ok(())
}

fn infer_change_title(capsule: &Path, prompt: &str) -> Result<String> {
    validate_pi()?;
    if prompt.trim().is_empty() {
        bail!("cannot infer a title from an empty prompt");
    }

    let value = run_structured_worker(
        &capsule.join("workspace"),
        CHANGE_PROMPT,
        CHANGE_OUTPUT_SCHEMA,
        prompt,
        "structured_output",
        "Change naming",
    )?;
    validate_change_title(
        value["change"]
            .as_str()
            .context("Pi returned invalid structured Change output")?,
    )
    .map(str::to_owned)
}

fn validate_change_title(title: &str) -> Result<&str> {
    let title = title.trim();
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
    Ok(title)
}

fn run_structured_worker(
    cwd: &Path,
    system_prompt: &str,
    schema: &str,
    prompt: &str,
    tools: &str,
    action: &str,
) -> Result<serde_json::Value> {
    let extension = materialize_extension(
        STRUCTURED_OUTPUT_EXTENSION,
        "grove-structured-output-extension",
    )?;
    let result = run_structured_worker_with_extension(
        cwd,
        system_prompt,
        schema,
        prompt,
        tools,
        action,
        &extension,
    );
    let _ = fs::remove_file(extension);
    result
}

fn run_structured_worker_with_extension(
    cwd: &Path,
    system_prompt: &str,
    schema: &str,
    prompt: &str,
    tools: &str,
    action: &str,
    extension: &Path,
) -> Result<serde_json::Value> {
    let mut child = Command::new("pi")
        .args(["--mode", "rpc", "--no-session", "--model", WORKER_MODEL])
        .args(["--thinking", "minimal", "--tools", tools])
        .args([
            "--no-context-files",
            "--no-skills",
            "--no-prompt-templates",
            "--no-extensions",
        ])
        .args(["--system-prompt", system_prompt])
        .arg("--extension")
        .arg(extension)
        .args(["--structured-output-schema", schema])
        .current_dir(cwd)
        .env_remove("GROVE_DIRECTIVE_CD_FILE")
        .env_remove(ACTIVITY_CAPABILITY)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to launch Pi {action} worker"))?;

    let mut stdin = child.stdin.take().context("Pi RPC worker has no input")?;
    let stdout = child.stdout.take().context("Pi RPC worker has no output")?;
    let mut lines = BufReader::new(stdout).lines();
    let result = (|| -> Result<Option<serde_json::Value>> {
        send_rpc_prompt(&mut stdin, prompt)?;
        let mut attempts = 1;
        let mut result = None;
        let mut last_response = None;
        for line in lines.by_ref() {
            let line = line.with_context(|| format!("failed to read Pi {action} RPC output"))?;
            let event: serde_json::Value = serde_json::from_str(&line)
                .with_context(|| format!("Pi {action} worker returned invalid RPC JSONL"))?;
            if event["type"] == "response"
                && event["command"] == "prompt"
                && event["success"] == false
            {
                bail!("Pi {action} worker rejected the prompt");
            }
            if event["type"] == "message_end"
                && event["message"]["role"] == "assistant"
                && event["message"]["stopReason"] == "error"
            {
                let message = event["message"]["errorMessage"]
                    .as_str()
                    .unwrap_or("provider request failed");
                bail!("Pi {action} worker failed: {message}");
            }
            if event["type"] == "message_end" && event["message"]["role"] == "assistant" {
                last_response = event["message"]["content"]
                    .as_array()
                    .and_then(|content| {
                        content
                            .iter()
                            .find_map(|part| part["text"].as_str().map(str::to_owned))
                    });
            }
            if event["type"] == "tool_execution_end"
                && event["toolName"] == "structured_output"
                && event["isError"] != true
            {
                if result.is_some() {
                    bail!("Pi {action} worker returned multiple structured outputs");
                }
                result = Some(event["result"]["details"].clone());
            }
            if event["type"] == "agent_settled" {
                if result.is_some() {
                    return Ok(result);
                }
                if attempts == 2 {
                    if let Some(response) = last_response {
                        return serde_json::from_str(&response)
                            .map(Some)
                            .with_context(|| {
                                format!(
                                    "Pi {action} worker returned invalid JSON instead of structured output"
                                )
                            });
                    }
                    return Ok(None);
                }
                attempts += 1;
                send_rpc_prompt(
                    &mut stdin,
                    "Return the result now by calling structured_output with the required schema.",
                )?;
            }
        }
        Ok(None)
    })();

    drop(stdin);
    drop(lines);
    let _ = child.kill();
    child.wait().context("failed to wait for Pi RPC worker")?;
    result?.with_context(|| format!("Pi {action} worker returned no structured output"))
}

fn send_rpc_prompt(stdin: &mut impl Write, prompt: &str) -> Result<()> {
    serde_json::to_writer(
        &mut *stdin,
        &serde_json::json!({"type": "prompt", "message": prompt}),
    )
    .context("failed to encode Pi RPC prompt")?;
    stdin
        .write_all(b"\n")
        .context("failed to write Pi RPC prompt")?;
    stdin.flush().context("failed to flush Pi RPC prompt")
}

fn validate_ship_metadata(metadata: &ShipMetadata) -> Result<()> {
    if let Some(commit) = &metadata.commit {
        validate_subject(commit, "commit")?;
    }
    if let Some(pull_request) = &metadata.pull_request {
        validate_subject(&pull_request.title, "pull request title")?;
        if pull_request.body.trim().is_empty()
            || pull_request.body.len() > 1_000
            || pull_request
                .body
                .chars()
                .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            bail!("Pi returned an invalid pull request body");
        }
    }
    Ok(())
}

fn validate_subject(subject: &str, label: &str) -> Result<()> {
    let subject = subject.trim();
    if subject.is_empty()
        || subject.len() > 100
        || subject.contains(['\r', '\n'])
        || !subject.contains(": ")
    {
        bail!("Pi returned an invalid {label}");
    }
    Ok(())
}

fn materialize_extension(contents: &[u8], label: &str) -> Result<PathBuf> {
    let temporary = temporary_path(&env::temp_dir(), label).with_extension("ts");
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .with_context(|| format!("failed to create {}", temporary.display()))?;
    file.write_all(contents)
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
    let seed = format!("{}-{nonce}", std::process::id());
    let digest = blake3::hash(seed.as_bytes()).to_hex();
    parent.join(format!(".{label}-{}", &digest[..8]))
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
