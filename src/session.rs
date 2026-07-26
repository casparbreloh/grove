use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, de::DeserializeOwned};

use crate::change;

const CHANGE_SESSION_EXTENSION: &[u8] = include_bytes!("extensions/change-session.ts");
const STRUCTURED_OUTPUT_EXTENSION: &[u8] = include_bytes!("extensions/structured-output.ts");
const WORKER_MODEL: &str = "openai-codex/gpt-5.6-sol";
const CHANGE_PROMPT: &str = "Create a concise title of exactly three or four words for the user's request. Call structured_output with the title in the change field. Do not answer in any other way.";
const CHANGE_OUTPUT_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "change": { "type": "string", "minLength": 1, "maxLength": 80 }
  },
  "required": ["change"],
  "additionalProperties": false
}"#;
const SHIP_PROMPT: &str = "Write concise publication metadata for the supplied Change. Set commit to null when Published history is false because Grove uses the new pull-request title for the initial commit. Otherwise, a commit is a single Conventional Commit subject describing only newly staged work, with no body. A pull-request title is also a single Conventional Commit subject, but describes the complete Change. Return pull-request metadata only when a pull request must be created or its current title and body no longer fit. Treat the supplied summary as an index of the complete publication. Use it as primary context. If intent or scope is unclear, use read selectively until you understand the Change well enough to name it accurately. Avoid unrelated investigation. Finish only by calling structured_output.";
const SHIP_OUTPUT_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "commit": { "anyOf": [{ "type": "string", "minLength": 1, "maxLength": 100, "pattern": "^.+: .+$" }, { "type": "null" }] },
    "pull_request": {
      "anyOf": [
        {
          "type": "object",
          "properties": {
            "title": { "type": "string", "minLength": 1, "maxLength": 100, "pattern": "^.+: .+$" },
            "body": { "type": "string", "minLength": 1, "maxLength": 1000 }
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangeTitle {
    change: String,
}

pub(crate) fn require_pi() -> Result<()> {
    validate_pi()
}

pub(crate) fn attach(workspace: &Path) -> Result<()> {
    validate_pi()?;
    let capsule = workspace
        .parent()
        .context("Grove workspace has no Change capsule")?;
    let _lock = change::lock(capsule)?;
    let sessions = capsule.join("pi");
    create_private_directory_all(&sessions).with_context(|| {
        format!(
            "failed to create Pi session directory {}",
            sessions.display()
        )
    })?;
    let executable = env::current_exe().context("failed to locate the Grove executable")?;
    let change_id = capsule
        .file_name()
        .and_then(|name| name.to_str())
        .context("Change capsule has no valid ID")?;
    let extension = materialize_extension(CHANGE_SESSION_EXTENSION, "grove-session")?;
    let status = Command::new("pi")
        .arg("--session-dir")
        .arg(&sessions)
        .arg("--continue")
        .arg("--extension")
        .arg(extension.path())
        .current_dir(workspace)
        .env("GROVE_EXECUTABLE", executable)
        .env("GROVE_CHANGE_ID", change_id)
        .env("GROVE_CHANGE_CAPSULE", capsule)
        .env_remove("GROVE_DIRECTIVE_CD_FILE")
        .status()
        .with_context(|| format!("failed to launch Pi in {}", workspace.display()))?;
    if !status.success() {
        bail!("Pi exited with {status} in {}", workspace.display());
    }
    Ok(())
}

pub(crate) fn generate_ship_metadata(workspace: &Path, summary: &str) -> Result<ShipMetadata> {
    validate_pi()?;
    let metadata = run_structured_worker(
        workspace,
        SHIP_PROMPT,
        SHIP_OUTPUT_SCHEMA,
        summary,
        "read,structured_output",
        "shipping metadata",
    )?;
    validate_ship_metadata(&metadata)?;
    Ok(metadata)
}

pub(crate) fn name_change(change_id: &str) -> Result<()> {
    let capsule = change_capsule(change_id)?;
    capsule.validate_identity()?;
    let mut prompt = String::new();
    std::io::stdin()
        .read_to_string(&mut prompt)
        .context("failed to read the title prompt")?;
    let mut last_error = None;
    for delay in [0, 250, 1_000] {
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
        match infer_change_title(&capsule, &prompt) {
            Ok(title) => {
                println!("{title}");
                return Ok(());
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.expect("title inference attempted at least once"))
}

pub(crate) fn apply_change_title(change_id: &str) -> Result<()> {
    let capsule = change_capsule(change_id)?;
    let mut title = String::new();
    std::io::stdin()
        .read_to_string(&mut title)
        .context("failed to read the inferred title")?;
    let title = validate_change_title(&title)?;
    capsule.initialize_title(title)
}

fn change_capsule(change_id: &str) -> Result<change::Capsule> {
    let path = env::var_os("GROVE_CHANGE_CAPSULE")
        .map(PathBuf::from)
        .context("GROVE_CHANGE_CAPSULE is not set")?;
    change::Capsule::at(path, change_id.to_owned())
}

fn infer_change_title(capsule: &change::Capsule, prompt: &str) -> Result<String> {
    validate_pi()?;
    if prompt.trim().is_empty() {
        bail!("cannot infer a title from an empty prompt");
    }

    let output: ChangeTitle = run_structured_worker(
        &capsule.workspace(),
        CHANGE_PROMPT,
        CHANGE_OUTPUT_SCHEMA,
        prompt,
        "structured_output",
        "Change naming",
    )?;
    validate_change_title(&output.change).map(str::to_owned)
}

fn validate_change_title(title: &str) -> Result<&str> {
    let title = title.trim();
    let words = title.split_whitespace().collect::<Vec<_>>();
    if title.is_empty()
        || title.len() > 80
        || title.contains(['\r', '\n'])
        || !(3..=4).contains(&words.len())
        || !title.chars().all(change::display_safe)
        || words
            .iter()
            .any(|word| !word.chars().any(char::is_alphanumeric))
    {
        bail!("Pi returned an invalid title");
    }
    Ok(title)
}

fn run_structured_worker<T: DeserializeOwned>(
    cwd: &Path,
    system_prompt: &str,
    schema: &str,
    prompt: &str,
    tools: &str,
    action: &str,
) -> Result<T> {
    let extension = materialize_extension(
        STRUCTURED_OUTPUT_EXTENSION,
        "grove-structured-output-extension",
    )?;
    let mut child = Command::new("pi")
        .args(["--mode", "json", "--no-session", "--model", WORKER_MODEL])
        .args(["--thinking", "minimal", "--tools", tools])
        .args([
            "--no-context-files",
            "--no-skills",
            "--no-prompt-templates",
            "--no-extensions",
        ])
        .args(["--system-prompt", system_prompt])
        .arg("--extension")
        .arg(extension.path())
        .args(["--structured-output-schema", schema])
        .current_dir(cwd)
        .env_remove("GROVE_DIRECTIVE_CD_FILE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to launch Pi {action} worker"))?;

    let prompt_written = child
        .stdin
        .take()
        .context("Pi worker has no input")
        .and_then(|mut stdin| {
            stdin
                .write_all(prompt.as_bytes())
                .context("failed to write Pi worker prompt")
        });
    let output = child
        .wait_with_output()
        .with_context(|| format!("failed to wait for Pi {action} worker"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Pi {action} worker exited with {}: {}",
            output.status,
            stderr.trim()
        );
    }
    prompt_written?;

    let mut result = None;
    let mut last_response = None;
    let mut last_error = None;
    for line in output.stdout.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let event: serde_json::Value = serde_json::from_slice(line)
            .with_context(|| format!("Pi {action} worker returned invalid JSONL"))?;
        if event["type"] == "message_end" && event["message"]["role"] == "assistant" {
            if event["message"]["stopReason"] == "error" {
                last_error = Some(
                    event["message"]["errorMessage"]
                        .as_str()
                        .unwrap_or("provider request failed")
                        .to_owned(),
                );
            } else {
                last_response = event["message"]["content"].as_array().and_then(|content| {
                    content
                        .iter()
                        .find_map(|part| part["text"].as_str().map(str::to_owned))
                });
            }
        }
        if event["type"] == "tool_execution_end"
            && event["toolName"] == "structured_output"
            && event["isError"] != true
            && result.replace(event["result"]["details"].clone()).is_some()
        {
            bail!("Pi {action} worker returned multiple structured outputs");
        }
    }
    if let Some(value) = result {
        return serde_json::from_value(value)
            .with_context(|| format!("Pi returned invalid structured {action} output"));
    }
    if let Some(response) = last_response {
        return serde_json::from_str(&response)
            .with_context(|| format!("Pi {action} worker returned invalid JSON"));
    }
    if let Some(error) = last_error {
        bail!("Pi {action} worker failed: {error}");
    }
    bail!("Pi {action} worker returned no structured output")
}

fn validate_ship_metadata(metadata: &ShipMetadata) -> Result<()> {
    if let Some(commit) = &metadata.commit {
        validate_conventional_subject(commit, "commit")?;
    }
    if let Some(pull_request) = &metadata.pull_request {
        validate_conventional_subject(&pull_request.title, "pull request title")?;
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
    if subject.is_empty() || subject.len() > 100 || !subject.chars().all(change::display_safe) {
        bail!("Pi returned an invalid {label}");
    }
    Ok(())
}

fn validate_conventional_subject(subject: &str, label: &str) -> Result<()> {
    validate_subject(subject, label)?;
    if !subject.contains(": ") {
        bail!("Pi returned an invalid {label}");
    }
    Ok(())
}

struct TemporaryExtension {
    path: PathBuf,
}

impl TemporaryExtension {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryExtension {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn materialize_extension(contents: &[u8], label: &str) -> Result<TemporaryExtension> {
    let temporary = temporary_path(&env::temp_dir(), label).with_extension("ts");
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .with_context(|| format!("failed to create {}", temporary.display()))?;
    let extension = TemporaryExtension { path: temporary };
    file.write_all(contents)
        .with_context(|| format!("failed to write {}", extension.path().display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync {}", extension.path().display()))?;
    Ok(extension)
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
