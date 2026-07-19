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

const RECORD_VERSION: u8 = 3;

pub(crate) struct RepositoryDirectory {
    path: PathBuf,
}

impl RepositoryDirectory {
    pub(crate) fn new(name: String, common_dir: PathBuf) -> Result<Self> {
        let home = std::env::var_os("HOME").context("HOME is not set")?;
        let digest = blake3::hash(common_dir.as_os_str().as_encoded_bytes()).to_hex();
        Ok(Self {
            path: PathBuf::from(home)
                .join(".grove")
                .join(format!("{name}-{}", &digest[..8])),
        })
    }

    pub(crate) fn reserve(&self, creation: Creation) -> Result<Reserved> {
        Reserved::create(&self.path, creation)
    }

    pub(crate) fn records(&self) -> Result<Vec<(PathBuf, Record)>> {
        Record::load_all(&self.path)
    }

    pub(crate) fn record(&self, id: &str) -> Result<Option<(PathBuf, Record)>> {
        let capsule = self.path.join(id);
        Ok(Record::load_optional(&capsule.join("change.json"))?.map(|record| (capsule, record)))
    }
}

pub(crate) struct Lock {
    _file: File,
}

pub(crate) fn lock(capsule: &Path) -> Result<Lock> {
    try_lock(capsule)?.context("change is already open in another Grove process")
}

pub(crate) fn try_lock(capsule: &Path) -> Result<Option<Lock>> {
    let path = capsule.join(".activity.lock");
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let file = options
        .open(&path)
        .with_context(|| format!("failed to open change lock {}", path.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(Lock { _file: file })),
        Err(fs::TryLockError::WouldBlock) => Ok(None),
        Err(fs::TryLockError::Error(error)) => Err(error)
            .with_context(|| format!("failed to lock change capsule {}", capsule.display())),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Creation {
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
pub(crate) struct Closing {
    pub(crate) outcome: Outcome,
    pub(crate) tip_oid: String,
    pub(crate) target_oid: Option<String>,
    pub(crate) target_ref: Option<String>,
    pub(crate) local_branch: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Record {
    version: u8,
    pub(crate) id: String,
    pub(crate) title: Option<String>,
    pub(crate) state: State,
    pub(crate) created_at: u64,
    pub(crate) base_oid: String,
    pub(crate) parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) closing: Option<Closing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) archived_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) outcome: Option<Outcome>,
}

impl Record {
    fn load_all(root: &Path) -> Result<Vec<(PathBuf, Self)>> {
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

    fn load_optional(path: &Path) -> Result<Option<Self>> {
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
            bail!(
                "unsupported change record {} (version {}, expected {})",
                path.display(),
                record.version,
                RECORD_VERSION
            );
        }
        Ok(Some(record))
    }
}

pub(crate) struct Reserved {
    id: String,
    capsule: PathBuf,
}

impl Reserved {
    fn create(root: &Path, creation: Creation) -> Result<Self> {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before the Unix epoch")?
            .as_secs();
        create_private_directory_all(root)
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
                base_oid: creation.base_oid.clone(),
                parent: creation.parent.clone(),
                closing: None,
                archived_at: None,
                outcome: None,
            };
            if let Err(error) = replace_json(&capsule.join("change.json"), &record) {
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

    pub(crate) fn workspace(&self) -> PathBuf {
        self.capsule.join("workspace")
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
    capsule: PathBuf,
}

impl Change {
    pub(crate) fn workspace(&self) -> PathBuf {
        self.capsule.join("workspace")
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

pub(crate) fn mark_closing(capsule: &Path, expected_id: &str, closing: Closing) -> Result<()> {
    update_record(capsule, expected_id, |record| {
        if !matches!(record.state, State::Active) {
            bail!("change '{}' is not active", record.id);
        }
        record.state = State::Closing;
        record.closing = Some(closing);
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
        let closing = record
            .closing
            .take()
            .context("closing change has no closing facts")?;
        record.state = State::Archived;
        record.archived_at = Some(closed_at);
        record.outcome = Some(closing.outcome);
        Ok(())
    })
}

pub(crate) fn restore_active(capsule: &Path, expected_id: &str) -> Result<()> {
    update_record(capsule, expected_id, |record| {
        if !matches!(record.state, State::Closing) {
            bail!("change '{}' is not closing", record.id);
        }
        record.state = State::Active;
        record.closing = None;
        Ok(())
    })
}

fn update_record(
    capsule: &Path,
    expected_id: &str,
    update: impl FnOnce(&mut Record) -> Result<()>,
) -> Result<()> {
    let lock_path = capsule.join(".metadata.lock");
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

    replace_json(&path, &record)
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

fn replace_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().context("Grove record has no parent")?;
    let name = path
        .file_name()
        .context("Grove record has no file name")?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{name}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    if let Err(error) = write_json(&temporary, value) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error)
            .with_context(|| format!("failed to install Grove record {}", path.display()));
    }
    sync_parent(path)
}

fn sync_parent(path: &Path) -> Result<()> {
    let parent = path.parent().context("Grove record has no parent")?;
    File::open(parent)
        .with_context(|| format!("failed to open Grove directory {}", parent.display()))?
        .sync_all()
        .with_context(|| format!("failed to sync Grove directory {}", parent.display()))?;
    Ok(())
}
