use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const RECORD_VERSION: u8 = 1;
const REPOSITORY_RECORD_VERSION: u8 = 1;

#[derive(Deserialize, Serialize)]
struct RepositoryRecord {
    version: u8,
    name: String,
    git_common_dir: String,
}

pub(crate) fn locate_repository(root: &Path, name: &str, common_dir: &Path) -> Result<PathBuf> {
    let candidates = repository_candidates(root, name, common_dir);
    for candidate in &candidates {
        if repository_matches(candidate, common_dir)? {
            return Ok(candidate.clone());
        }
    }
    if !candidates[0].exists() {
        Ok(candidates[0].clone())
    } else {
        Ok(candidates[1].clone())
    }
}

pub(crate) fn claim_repository(root: &Path, name: &str, common_dir: &Path) -> Result<PathBuf> {
    create_private_directory_all(root)
        .with_context(|| format!("failed to create Grove repositories {}", root.display()))?;
    let _claim_lock = repository_claim_lock(root)?;
    let record = RepositoryRecord {
        version: REPOSITORY_RECORD_VERSION,
        name: name.to_owned(),
        git_common_dir: common_dir.to_string_lossy().into_owned(),
    };
    for candidate in repository_candidates(root, name, common_dir) {
        loop {
            if repository_matches(&candidate, common_dir)? {
                return Ok(candidate);
            }
            if candidate.exists() {
                break;
            }
            match create_private_directory(&candidate) {
                Ok(()) => {
                    if let Err(error) = install_repository_record(&candidate, &record) {
                        let _ = fs::remove_dir_all(&candidate);
                        return Err(error);
                    }
                    return Ok(candidate);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("failed to claim Grove repository {}", candidate.display())
                    });
                }
            }
        }
    }
    bail!("could not isolate Grove repository '{name}'")
}

fn repository_claim_lock(root: &Path) -> Result<File> {
    let path = root.join(".lock");
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let file = options
        .open(&path)
        .with_context(|| format!("failed to open repository claim lock {}", path.display()))?;
    file.lock()
        .with_context(|| format!("failed to lock Grove repositories {}", root.display()))?;
    Ok(file)
}

fn install_repository_record(path: &Path, record: &RepositoryRecord) -> Result<()> {
    let installed = path.join("repository.json");
    let temporary = path.join(format!(
        ".repository.json-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    if let Err(error) = write_json(&temporary, record) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, &installed) {
        let _ = fs::remove_file(&temporary);
        return Err(error).with_context(|| {
            format!(
                "failed to install repository record {}",
                installed.display()
            )
        });
    }
    sync_parent(&installed)
}

fn repository_candidates(root: &Path, name: &str, common_dir: &Path) -> [PathBuf; 2] {
    let digest = blake3::hash(common_dir.as_os_str().as_encoded_bytes()).to_hex();
    [
        root.join(name),
        root.join(format!("{name}-{}", &digest[..8])),
    ]
}

fn repository_matches(path: &Path, common_dir: &Path) -> Result<bool> {
    let manifest = path.join("repository.json");
    let bytes = match fs::read(&manifest) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error).with_context(|| {
                format!("failed to read repository record {}", manifest.display())
            });
        }
    };
    let record: RepositoryRecord = serde_json::from_slice(&bytes)
        .with_context(|| format!("invalid repository record {}", manifest.display()))?;
    if record.version != REPOSITORY_RECORD_VERSION {
        bail!("unsupported repository record {}", manifest.display());
    }
    Ok(Path::new(&record.git_common_dir) == common_dir)
}

pub(crate) struct Lock {
    _file: File,
}

pub(crate) fn lock(capsule: &Path) -> Result<Lock> {
    let path = capsule.join(".lock");
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let file = options
        .open(&path)
        .with_context(|| format!("failed to open change lock {}", path.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(Lock { _file: file }),
        Err(fs::TryLockError::WouldBlock) => {
            bail!("change is already open in another Grove process")
        }
        Err(fs::TryLockError::Error(error)) => Err(error)
            .with_context(|| format!("failed to lock change capsule {}", capsule.display())),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Creation {
    pub(crate) base_ref: Option<String>,
    pub(crate) base_oid: String,
    pub(crate) parent: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum State {
    Active,
    Closing,
    Archived,
}

impl State {
    pub(crate) fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    pub(crate) fn is_closing(&self) -> bool {
        matches!(self, Self::Closing)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Outcome {
    Integrated,
    Discarded,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Closure {
    pub(crate) closed_at: Option<u64>,
    pub(crate) outcome: Outcome,
    pub(crate) tip_oid: String,
    pub(crate) target_oid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target_ref: Option<String>,
    pub(crate) integration: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Record {
    pub(crate) version: u8,
    pub(crate) id: String,
    pub(crate) title: Option<String>,
    pub(crate) state: State,
    pub(crate) created_at: u64,
    pub(crate) repository: String,
    pub(crate) creation: Creation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) closure: Option<Closure>,
}

impl Record {
    pub(crate) fn load_all(root: &Path) -> Result<Vec<(PathBuf, Self)>> {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read Grove changes {}", root.display()));
            }
        };
        let mut records = Vec::new();
        for entry in entries {
            let entry = entry
                .with_context(|| format!("failed to read Grove change in {}", root.display()))?;
            if !entry
                .file_type()
                .with_context(|| format!("failed to inspect {}", entry.path().display()))?
                .is_dir()
            {
                continue;
            }
            let capsule = entry.path();
            let Some(record) = Self::load_optional(&capsule.join("change.json"))? else {
                continue;
            };
            if entry.file_name() != std::ffi::OsStr::new(&record.id) {
                bail!(
                    "change record ID does not match capsule {}",
                    capsule.display()
                );
            }
            records.push((capsule, record));
        }
        Ok(records)
    }

    pub(crate) fn load_optional(path: &Path) -> Result<Option<Self>> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read change record {}", path.display()));
            }
        };
        let record: Self = serde_json::from_slice(&bytes)
            .with_context(|| format!("invalid change record {}", path.display()))?;
        if record.version != RECORD_VERSION || !valid_id(&record.id) {
            bail!("unsupported change record {}", path.display());
        }
        Ok(Some(record))
    }
}

pub(crate) struct Reserved {
    id: String,
    capsule: PathBuf,
}

impl Reserved {
    pub(crate) fn create(root: &Path, repository: &Path, creation: Creation) -> Result<Self> {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before the Unix epoch")?
            .as_secs();
        fs::create_dir_all(root)
            .with_context(|| format!("failed to create Grove root {}", root.display()))?;
        for nonce in 0..100_u8 {
            let id = generate_id(root, nonce)?;
            let capsule = root.join(&id);
            match create_private_directory(&capsule) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("failed to reserve change capsule {}", capsule.display())
                    });
                }
            }
            let record = Record {
                version: RECORD_VERSION,
                id: id.clone(),
                title: None,
                state: State::Active,
                created_at,
                repository: repository.to_string_lossy().into_owned(),
                creation: creation.clone(),
                closure: None,
            };
            if let Err(error) = write_record(&capsule.join("change.json"), &record) {
                if let Err(rollback_error) = fs::remove_dir_all(&capsule) {
                    return Err(error).context(format!(
                        "record creation failed and capsule rollback also failed: {rollback_error}"
                    ));
                }
                return Err(error);
            }
            return Ok(Self { id, capsule });
        }
        bail!("could not reserve a unique Grove change")
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn worktree(&self) -> PathBuf {
        self.capsule.join("worktree")
    }

    pub(crate) fn finish(self) -> Change {
        Change {
            id: self.id,
            capsule: self.capsule,
        }
    }

    pub(crate) fn rollback(self) -> Result<()> {
        match fs::remove_dir_all(&self.capsule) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).with_context(|| {
                format!(
                    "failed to roll back change capsule {}",
                    self.capsule.display()
                )
            }),
        }
    }
}

pub(crate) struct Change {
    pub(crate) id: String,
    pub(crate) capsule: PathBuf,
}

impl Change {
    pub(crate) fn worktree(&self) -> PathBuf {
        self.capsule.join("worktree")
    }
}

pub(crate) fn initialize_title(capsule: &Path, expected_id: &str, title: &str) -> Result<()> {
    update_record(capsule, expected_id, |record| {
        if record.title.is_none() {
            record.title = Some(title.to_owned());
        }
        Ok(())
    })
}

pub(crate) fn mark_closing(capsule: &Path, expected_id: &str, closure: Closure) -> Result<()> {
    update_record(capsule, expected_id, |record| {
        if !matches!(record.state, State::Active) {
            bail!("change '{}' is not active", record.id);
        }
        record.state = State::Closing;
        record.closure = Some(closure);
        Ok(())
    })
}

pub(crate) fn mark_archived(capsule: &Path, expected_id: &str) -> Result<()> {
    update_record(capsule, expected_id, |record| {
        if !matches!(record.state, State::Closing) {
            bail!("change '{}' is not closing", record.id);
        }
        let closed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before the Unix epoch")?
            .as_secs();
        record
            .closure
            .as_mut()
            .context("closing change has no closure facts")?
            .closed_at = Some(closed_at);
        record.state = State::Archived;
        Ok(())
    })
}

pub(crate) fn restore_active(capsule: &Path, expected_id: &str) -> Result<()> {
    update_record(capsule, expected_id, |record| {
        if !matches!(record.state, State::Closing) {
            bail!("change '{}' is not closing", record.id);
        }
        record.state = State::Active;
        record.closure = None;
        Ok(())
    })
}

fn update_record(
    capsule: &Path,
    expected_id: &str,
    update: impl FnOnce(&mut Record) -> Result<()>,
) -> Result<()> {
    let lock_path = capsule.join(".record.lock");
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let lock = options
        .open(&lock_path)
        .with_context(|| format!("failed to open change record lock {}", lock_path.display()))?;
    lock.lock()
        .with_context(|| format!("failed to lock change record {}", capsule.display()))?;

    let path = capsule.join("change.json");
    let mut record = Record::load_optional(&path)?
        .with_context(|| format!("change record is missing from {}", capsule.display()))?;
    if record.id != expected_id {
        bail!("change identity does not match capsule record");
    }
    update(&mut record)?;

    let temporary = capsule.join(format!(
        ".change.json-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    if let Err(error) = write_record(&temporary, &record) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, &path) {
        let _ = fs::remove_file(&temporary);
        return Err(error)
            .with_context(|| format!("failed to install change record {}", path.display()));
    }
    sync_parent(&path)?;
    Ok(())
}

fn generate_id(root: &Path, nonce: u8) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_nanos();
    let seed = format!("{}:{now}:{}:{nonce}", root.display(), std::process::id());
    Ok(blake3::hash(seed.as_bytes()).to_hex()[..8].to_owned())
}

fn valid_id(id: &str) -> bool {
    id.len() == 8
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn create_private_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(path)
}

fn create_private_directory_all(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(path)
}

fn write_record(path: &Path, record: &Record) -> Result<()> {
    write_json(path, record)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(path)
        .with_context(|| format!("failed to create Grove record {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, value)
        .with_context(|| format!("failed to serialize Grove record {}", path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("failed to finish Grove record {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync Grove record {}", path.display()))?;
    sync_parent(path)?;
    Ok(())
}

fn sync_parent(path: &Path) -> Result<()> {
    let parent = path.parent().context("Grove record has no parent")?;
    File::open(parent)
        .with_context(|| format!("failed to open Grove directory {}", parent.display()))?
        .sync_all()
        .with_context(|| format!("failed to sync Grove directory {}", parent.display()))?;
    Ok(())
}
