use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthChar;

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

    pub(crate) fn inspect(&self) -> Result<RepositoryInspection> {
        let mut inspection = RepositoryInspection::default();
        let metadata = match fs::symlink_metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(inspection),
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to inspect Grove changes {}", self.path.display())
                });
            }
        };
        if !metadata.file_type().is_dir() {
            inspection.findings.push(format!(
                "Grove repository directory is not a directory: {}",
                self.path.display()
            ));
            return Ok(inspection);
        }
        inspect_private_directory(
            &self.path,
            "Grove repository directory",
            &mut inspection.findings,
        );
        let entries = fs::read_dir(&self.path)
            .with_context(|| format!("failed to read Grove changes {}", self.path.display()))?;
        let mut entries = entries
            .collect::<std::io::Result<Vec<_>>>()
            .with_context(|| format!("failed to read Grove change in {}", self.path.display()))?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let file_type = entry
                .file_type()
                .with_context(|| format!("failed to inspect {}", path.display()))?;
            if !file_type.is_dir() {
                inspection
                    .findings
                    .push(format!("{name}: capsule is not a directory"));
                continue;
            }
            inspect_private_directory(&path, &name, &mut inspection.findings);
            for lock in [".activity.lock", ".metadata.lock", ".mutation.lock"] {
                inspect_lock(&path.join(lock), &name, &mut inspection.findings);
            }
            let record_path = path.join("change.json");
            if !inspect_private_file(&record_path, &name, "change.json", &mut inspection.findings) {
                continue;
            }
            let record = match Record::load_optional(&record_path) {
                Ok(Some(record)) => record,
                Ok(None) => {
                    inspection
                        .findings
                        .push(format!("{name}: change.json is missing"));
                    continue;
                }
                Err(error) => {
                    inspection.findings.push(format!("{name}: {error:#}"));
                    continue;
                }
            };
            if entry.file_name() != std::ffi::OsStr::new(&record.id) {
                inspection.findings.push(format!(
                    "{name}: Change ID '{}' does not match the capsule name",
                    record.id
                ));
                continue;
            }
            inspection.records.push((path, record));
        }
        Ok(inspection)
    }
}

#[derive(Default)]
pub(crate) struct RepositoryInspection {
    pub(crate) records: Vec<(PathBuf, Record)>,
    pub(crate) findings: Vec<String>,
}

#[cfg(unix)]
fn inspect_private_directory(path: &Path, label: &str, findings: &mut Vec<String>) {
    if let Ok(metadata) = fs::symlink_metadata(path)
        && metadata.permissions().mode() & 0o077 != 0
    {
        findings.push(format!(
            "{label}: unsafe permissions {:03o} on {}",
            metadata.permissions().mode() & 0o777,
            path.display()
        ));
    }
}

#[cfg(not(unix))]
fn inspect_private_directory(_path: &Path, _label: &str, _findings: &mut Vec<String>) {}

fn inspect_private_file(path: &Path, label: &str, kind: &str, findings: &mut Vec<String>) -> bool {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
        Err(error) => {
            findings.push(format!("{label}: cannot inspect {kind}: {error}"));
            return false;
        }
    };
    if !metadata.file_type().is_file() {
        findings.push(format!("{label}: {kind} is not a regular file"));
        return false;
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o077 != 0 {
        findings.push(format!(
            "{label}: unsafe permissions {:03o} on {kind}",
            metadata.permissions().mode() & 0o777
        ));
    }
    true
}

fn inspect_lock(path: &Path, label: &str, findings: &mut Vec<String>) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            findings.push(format!(
                "{label}: cannot inspect {}: {error}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
            return;
        }
    };
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if !metadata.file_type().is_file() {
        findings.push(format!("{label}: {name} is not a regular file"));
        return;
    }
    inspect_private_file(path, label, &name, findings);
    if let Err(error) = OpenOptions::new().read(true).write(true).open(path) {
        findings.push(format!("{label}: cannot open {name}: {error}"));
    }
}

pub(crate) struct ActivityLock {
    _file: File,
}

pub(crate) struct MutationLock {
    _file: File,
}

pub(crate) fn lock(capsule: &Path) -> Result<ActivityLock> {
    try_lock(capsule)?.context("Change is already open in another Grove process")
}

pub(crate) fn try_lock(capsule: &Path) -> Result<Option<ActivityLock>> {
    Ok(try_lock_file(capsule, ".activity.lock")?.map(|file| ActivityLock { _file: file }))
}

pub(crate) fn try_lock_mutation(capsule: &Path) -> Result<Option<MutationLock>> {
    Ok(try_lock_file(capsule, ".mutation.lock")?.map(|file| MutationLock { _file: file }))
}

fn try_lock_file(capsule: &Path, name: &str) -> Result<Option<File>> {
    let path = capsule.join(name);
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let file = options
        .open(&path)
        .with_context(|| format!("failed to open change lock {}", path.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
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

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
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
#[serde(tag = "state", rename_all = "lowercase")]
enum Lifecycle {
    Active,
    Closing { closing: Closing },
    Archived { archived_at: u64, outcome: Outcome },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Record {
    pub(crate) id: String,
    pub(crate) title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) publication_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) published_oid: Option<String>,
    #[serde(flatten)]
    lifecycle: Lifecycle,
    pub(crate) created_at: u64,
    pub(crate) base_oid: String,
    pub(crate) parent: Option<String>,
}

impl Record {
    pub(crate) fn is_active(&self) -> bool {
        matches!(self.lifecycle, Lifecycle::Active)
    }

    pub(crate) fn is_closing(&self) -> bool {
        matches!(self.lifecycle, Lifecycle::Closing { .. })
    }

    pub(crate) fn is_archived(&self) -> bool {
        matches!(self.lifecycle, Lifecycle::Archived { .. })
    }

    pub(crate) fn closing(&self) -> Option<&Closing> {
        match &self.lifecycle {
            Lifecycle::Closing { closing } => Some(closing),
            Lifecycle::Active | Lifecycle::Archived { .. } => None,
        }
    }

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
            let record = match Self::load_optional(&capsule.join("change.json")) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(error) if error.downcast_ref::<std::io::Error>().is_none() => continue,
                Err(error) => return Err(error),
            };
            if entry.file_name() == std::ffi::OsStr::new(&record.id) {
                records.push((capsule, record));
            }
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
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("invalid change record {}", path.display()))?;
        if value.get("version").is_some() {
            bail!(
                "versioned change record requires explicit conversion: {}",
                path.display()
            );
        }
        let record: Self = serde_json::from_value(value)
            .with_context(|| format!("invalid change record {}", path.display()))?;
        if !valid_id(&record.id) {
            bail!("invalid Change ID in record {}", path.display());
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
                id: id.clone(),
                title: None,
                publication_branch: None,
                published_oid: None,
                lifecycle: Lifecycle::Active,
                created_at,
                base_oid: creation.base_oid.clone(),
                parent: creation.parent.clone(),
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

    pub(crate) fn finish(self) -> Capsule {
        Capsule {
            path: self.capsule,
            id: self.id,
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

pub(crate) struct Capsule {
    path: PathBuf,
    id: String,
}

impl Capsule {
    pub(crate) fn at(path: PathBuf, id: String) -> Result<Self> {
        if path.file_name() != Some(std::ffi::OsStr::new(&id)) {
            bail!("change identity does not match capsule path");
        }
        Ok(Self { path, id })
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn workspace(&self) -> PathBuf {
        self.path.join("workspace")
    }

    pub(crate) fn lock_mutation(&self) -> Result<MutationLock> {
        try_lock_mutation(&self.path)?.context("Change is being modified by another Grove process")
    }

    pub(crate) fn validate_identity(&self) -> Result<()> {
        let path = self.path.join("change.json");
        let record = Record::load_optional(&path)?
            .with_context(|| format!("change record is missing from {}", self.path.display()))?;
        if record.id != self.id {
            bail!("change identity does not match capsule record");
        }
        Ok(())
    }

    pub(crate) fn initialize_title(&self, title: &str) -> Result<()> {
        self.update_record(|record| {
            if record.title.is_none() {
                record.title = Some(title.to_owned());
            }
            Ok(())
        })
    }

    pub(crate) fn initialize_publication_branch(&self, branch: &str) -> Result<()> {
        self.update_record(|record| match record.publication_branch.as_deref() {
            Some(existing) if existing != branch => {
                bail!("Change '{}' already publishes from '{existing}'", record.id)
            }
            Some(_) => Ok(()),
            None => {
                record.publication_branch = Some(branch.to_owned());
                Ok(())
            }
        })
    }

    pub(crate) fn mark_published(&self, branch: &str, oid: &str) -> Result<()> {
        self.update_record(|record| {
            if record.publication_branch.as_deref() != Some(branch) {
                bail!("Change '{}' publication branch changed", record.id);
            }
            record.published_oid = Some(oid.to_owned());
            Ok(())
        })
    }

    pub(crate) fn mark_closing(&self, closing: Closing) -> Result<()> {
        self.update_record(|record| {
            if !record.is_active() {
                bail!("Change '{}' is not active", record.id);
            }
            record.lifecycle = Lifecycle::Closing { closing };
            Ok(())
        })
    }

    pub(crate) fn mark_archived(&self) -> Result<()> {
        self.update_record(|record| {
            let Lifecycle::Closing { closing } = &record.lifecycle else {
                bail!("Change '{}' is not closing", record.id);
            };
            let archived_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("system clock is before the Unix epoch")?
                .as_secs();
            record.lifecycle = Lifecycle::Archived {
                archived_at,
                outcome: closing.outcome,
            };
            Ok(())
        })
    }

    pub(crate) fn restore_active(&self) -> Result<()> {
        self.update_record(|record| {
            if !record.is_closing() {
                bail!("Change '{}' is not closing", record.id);
            }
            record.lifecycle = Lifecycle::Active;
            Ok(())
        })
    }

    fn update_record(&self, update: impl FnOnce(&mut Record) -> Result<()>) -> Result<()> {
        let lock_path = self.path.join(".metadata.lock");
        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let lock = options.open(&lock_path).with_context(|| {
            format!("failed to open change record lock {}", lock_path.display())
        })?;
        lock.lock()
            .with_context(|| format!("failed to lock change record {}", self.path.display()))?;

        let path = self.path.join("change.json");
        let mut record = Record::load_optional(&path)?
            .with_context(|| format!("change record is missing from {}", self.path.display()))?;
        if record.id != self.id {
            bail!("change identity does not match capsule record");
        }
        update(&mut record)?;
        replace_json(&path, &record)
    }
}

pub(crate) fn publication_branch_base(title: &str) -> Result<String> {
    let mut branch = String::new();
    let mut separator = false;
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !branch.is_empty() {
                branch.push('-');
            }
            branch.push(character.to_ascii_lowercase());
            separator = false;
        } else if !branch.is_empty() {
            separator = true;
        }
    }
    if branch.is_empty() {
        bail!("Change Title cannot form an ASCII publication branch");
    }
    Ok(branch)
}

pub(crate) fn title_labels<'a>(
    changes: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
    reserved_titles: &[&str],
) -> Vec<String> {
    let changes = changes
        .into_iter()
        .map(|(id, title)| {
            (
                id,
                title.map(display_text).filter(|title| !title.is_empty()),
            )
        })
        .collect::<Vec<_>>();
    let mut title_counts = HashMap::new();
    for title in reserved_titles
        .iter()
        .copied()
        .chain(changes.iter().filter_map(|(_, title)| title.as_deref()))
    {
        *title_counts.entry(title.to_owned()).or_insert(0_usize) += 1;
    }
    changes
        .into_iter()
        .map(|(id, title)| match title {
            Some(title) if title_counts.get(title.as_str()) == Some(&1) => title,
            Some(title) => format!("{title} · {id}"),
            None => format!("Untitled · {id}"),
        })
        .collect()
}

pub(crate) fn display_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| display_safe(*character))
        .collect()
}

pub(crate) fn display_safe(character: char) -> bool {
    !character.is_control()
        && UnicodeWidthChar::width(character).is_some()
        && !matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
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
