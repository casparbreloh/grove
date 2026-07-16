use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::{
    ffi::OsString,
    os::unix::{ffi::OsStringExt, fs::DirBuilderExt, fs::OpenOptionsExt},
};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::change::{
    Change, Closure, Creation, Outcome, Record, Reserved, claim_repository, locate_repository,
    lock as lock_change, mark_archived, mark_closing, restore_active,
};

#[derive(Clone)]
pub(crate) struct Git {
    cwd: PathBuf,
}

#[derive(Debug)]
struct Worktree {
    path: PathBuf,
    branch: Option<String>,
    locked: bool,
    prunable: bool,
}

pub(crate) struct Status {
    pub(crate) changed: bool,
    pub(crate) added: usize,
    pub(crate) deleted: usize,
    pub(crate) conflicts: usize,
}

pub(crate) struct Divergence {
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
}

pub(crate) enum WorktreeState {
    Present(Status),
    Missing,
}

pub(crate) struct WorktreeView {
    pub(crate) id: String,
    pub(crate) title: Option<String>,
    pub(crate) path: PathBuf,
    pub(crate) base: String,
    pub(crate) divergence: Option<Divergence>,
    pub(crate) state: WorktreeState,
    pub(crate) current: bool,
}

pub(crate) struct Removal {
    pub(crate) navigate_to: Option<PathBuf>,
}

pub(crate) struct PreparedRemoval {
    path: PathBuf,
    branch: String,
    capsule: PathBuf,
    base_oid: String,
    primary: PathBuf,
    current: PathBuf,
    expected_oid: String,
    target_oid: Option<String>,
    target_ref: Option<String>,
    integration: String,
    force: bool,
}

#[derive(Serialize)]
struct ArchiveStats {
    version: u8,
    base_oid: String,
    final_tree: String,
    patch_digest: String,
    summary: FileSummary,
    files: Vec<ArchiveFile>,
}

#[derive(Default, Serialize)]
struct FileSummary {
    files_changed: usize,
    additions: u64,
    deletions: u64,
    added: usize,
    modified: usize,
    deleted: usize,
    renamed: usize,
    copied: usize,
    binary: usize,
}

#[derive(Serialize)]
struct ArchiveFile {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    old_path: Option<String>,
    status: String,
    additions: Option<u64>,
    deletions: Option<u64>,
    binary: bool,
}

struct BranchBase {
    display: String,
    divergence_ref: Option<String>,
    removal_ref: Option<String>,
    valid: bool,
}

struct Lineage {
    base_ref: Option<String>,
    base_oid: Option<String>,
    parent: Option<String>,
    default: bool,
}

impl Lineage {
    fn resolve_creation_base(git: &Git, source: Option<&str>) -> Result<Creation> {
        let Some(source) = source else {
            let default = git.default_branch()?;
            return Ok(Creation {
                base_oid: git.peel_commit(&default)?,
                base_ref: None,
                parent: Some(default),
            });
        };

        if source == "@" {
            let base_oid = git.peel_commit("HEAD")?;
            let parent = git
                .text(&["symbolic-ref", "--quiet", "--short", "HEAD"])
                .ok();
            return Ok(Creation {
                base_ref: Some(source.to_owned()),
                base_oid,
                parent,
            });
        }

        let base_oid = git.peel_commit(source)?;
        let parent = git.local_branch(source)?;
        Ok(Creation {
            base_ref: Some(source.to_owned()),
            base_oid,
            parent,
        })
    }

    fn load(git: &Git, branch: &str) -> Result<Self> {
        let path = git.changes_root()?.join(branch).join("change.json");
        let Some(record) = Record::load_optional(&path)? else {
            return Ok(Self {
                base_ref: None,
                base_oid: None,
                parent: None,
                default: false,
            });
        };
        Ok(Self {
            default: record.creation.base_ref.is_none(),
            base_ref: record.creation.base_ref,
            base_oid: Some(record.creation.base_oid),
            parent: record.creation.parent,
        })
    }

    fn base(&self, git: &Git, branch: &str) -> Result<BranchBase> {
        let (default_name, default_ref) = git.normalized_default()?;
        if branch == default_name {
            return Ok(BranchBase {
                display: String::new(),
                divergence_ref: None,
                removal_ref: None,
                valid: true,
            });
        }

        if self.default {
            return Ok(BranchBase {
                display: default_name,
                divergence_ref: Some(default_ref.clone()),
                removal_ref: Some(default_ref),
                valid: true,
            });
        }

        if self.lineage_is_empty() {
            return Ok(BranchBase {
                display: default_name,
                divergence_ref: Some(default_ref.clone()),
                removal_ref: Some(default_ref),
                valid: true,
            });
        }

        let lineage_is_valid = self
            .base_ref
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            && self
                .base_oid
                .as_deref()
                .is_some_and(|value| git.is_full_commit(value))
            && self.parent.as_deref().is_none_or(|value| !value.is_empty());
        if !lineage_is_valid {
            return Ok(BranchBase {
                display: "invalid lineage".to_owned(),
                divergence_ref: None,
                removal_ref: None,
                valid: false,
            });
        }

        let display_ref = self
            .base_ref
            .clone()
            .context("validated base ref is missing")?;
        let oid = self
            .base_oid
            .clone()
            .context("validated base OID is missing")?;
        if let Some(parent) = &self.parent {
            let live_parent = git.branch_exists(parent)? && git.is_ancestor(&oid, parent)?;
            if live_parent {
                return Ok(BranchBase {
                    display: parent.clone(),
                    divergence_ref: Some(parent.clone()),
                    removal_ref: Some(parent.clone()),
                    valid: true,
                });
            }
            return Ok(BranchBase {
                display: abbreviate_oid(&oid),
                divergence_ref: Some(oid),
                removal_ref: Some(default_ref),
                valid: true,
            });
        }

        Ok(BranchBase {
            display: display_ref,
            divergence_ref: Some(oid.clone()),
            removal_ref: Some(oid),
            valid: true,
        })
    }

    fn lineage_is_empty(&self) -> bool {
        self.base_ref.is_none() && self.base_oid.is_none() && self.parent.is_none()
    }
}

impl Git {
    pub(crate) fn discover() -> Result<Self> {
        Self::at(&std::env::current_dir()?)
    }

    pub(crate) fn at(cwd: &Path) -> Result<Self> {
        let cwd = cwd.to_owned();
        let git = Self { cwd };
        git.text(&["rev-parse", "--git-dir"])
            .context("not inside a Git repository")?;
        if git.text(&["rev-parse", "--is-bare-repository"])? == "true" {
            bail!("bare repositories are not supported");
        }
        Ok(git)
    }

    pub(crate) fn primary_path(&self) -> Result<PathBuf> {
        self.worktrees()?
            .into_iter()
            .next()
            .map(|worktree| worktree.path)
            .context("repository has no worktrees")
    }

    pub(crate) fn create_change(&self, from: Option<&str>) -> Result<Change> {
        let creation = Lineage::resolve_creation_base(self, from)?;
        let common_dir = self.common_dir()?;
        let (repositories, name) = self.repository_storage()?;
        let root = claim_repository(&repositories, &name, &common_dir)?;
        for _ in 0..100 {
            let reserved = Reserved::create(&root, &common_dir, creation.clone())?;
            let id = reserved.id().to_owned();
            match self.branch_exists(&id) {
                Ok(true) => {
                    reserved.rollback()?;
                    continue;
                }
                Ok(false) => {}
                Err(error) => {
                    reserved.rollback()?;
                    return Err(error).context("could not inspect reserved change branch");
                }
            }
            if let Err(error) = self.reserve_branch(&id, &creation.base_oid) {
                if let Err(rollback_error) = reserved.rollback() {
                    return Err(error).context(format!(
                        "branch reservation failed and capsule rollback also failed: {rollback_error:#}"
                    ));
                }
                let collision = self.branch_exists(&id)?;
                if collision {
                    continue;
                }
                return Err(error).context("could not reserve change branch");
            }
            let path = reserved.worktree();
            if let Err(error) = self.worktree_add(&path, &id) {
                let git_rollback = self.rollback_created_worktree(&path, &id);
                let capsule_rollback = reserved.rollback();
                if let Err(rollback_error) = git_rollback {
                    return Err(error).context(format!(
                        "worktree creation failed and Git rollback also failed: {rollback_error:#}"
                    ));
                }
                if let Err(rollback_error) = capsule_rollback {
                    return Err(error).context(format!(
                        "worktree creation failed and capsule rollback also failed: {rollback_error:#}"
                    ));
                }
                return Err(error).context("could not create change worktree");
            }
            return Ok(reserved.finish());
        }
        bail!("could not reserve a unique Grove change branch")
    }

    pub(crate) fn inventory(&self) -> Result<Vec<WorktreeView>> {
        let worktrees = self.worktrees()?;
        let current = self.current_root()?;
        let common_dir = self.common_dir()?;
        let changes_root = self.changes_root()?;
        let mut records = Vec::new();
        for (capsule, record) in Record::load_all(&changes_root)? {
            if !record.state.is_active() {
                continue;
            }
            if Path::new(&record.repository) != common_dir {
                bail!(
                    "change record {} belongs to a different repository",
                    capsule.join("change.json").display()
                );
            }
            records.push((record.created_at, capsule, record));
        }
        records.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.2.id.cmp(&right.2.id))
        });

        records
            .into_iter()
            .map(|(_, capsule, record)| {
                let expected_path = capsule.join("worktree");
                let worktree = worktrees
                    .iter()
                    .find(|worktree| worktree.branch.as_deref() == Some(&record.id))
                    .with_context(|| {
                        format!("active change {} has no linked worktree", record.id)
                    })?;
                let actual_path = if worktree.path.exists() {
                    worktree.path.canonicalize().with_context(|| {
                        format!("failed to resolve worktree {}", worktree.path.display())
                    })?
                } else {
                    worktree.path.clone()
                };
                let expected_path = if expected_path.exists() {
                    expected_path.canonicalize().with_context(|| {
                        format!("failed to resolve worktree {}", expected_path.display())
                    })?
                } else {
                    expected_path
                };
                if actual_path != expected_path {
                    bail!(
                        "active change {} is linked at unexpected worktree {}",
                        record.id,
                        worktree.path.display()
                    );
                }

                let base = Lineage::load(self, &record.id)?.base(self, &record.id)?;
                let divergence = base
                    .divergence_ref
                    .as_deref()
                    .map(|reference| self.divergence(&worktree.path, reference))
                    .transpose()?;
                let state = if worktree.prunable {
                    WorktreeState::Missing
                } else {
                    WorktreeState::Present(self.status(&worktree.path)?)
                };
                Ok(WorktreeView {
                    id: record.id,
                    title: record.title,
                    current: worktree.path == current,
                    path: worktree.path.clone(),
                    base: base.display,
                    divergence,
                    state,
                })
            })
            .collect()
    }

    pub(crate) fn current_path(&self) -> Result<PathBuf> {
        self.current_root()
    }

    pub(crate) fn recover_closing_removals(&self) -> Result<usize> {
        let worktrees = self.worktrees()?;
        let primary = worktrees
            .first()
            .context("repository has no worktrees")?
            .path
            .clone();
        let root = self.changes_root()?;
        let mut finalized = 0;
        for (capsule, record) in Record::load_all(&root)? {
            if !record.state.is_closing() {
                continue;
            }
            let _lock = lock_change(&capsule)?;
            let Some(record) = Record::load_optional(&capsule.join("change.json"))? else {
                continue;
            };
            if !record.state.is_closing() {
                continue;
            }
            let closure = record
                .closure
                .context("closing change has no closure facts")?;
            if let Some(worktree) = worktrees
                .iter()
                .find(|worktree| worktree.branch.as_deref() == Some(&record.id))
            {
                if !worktree.path.exists() {
                    bail!("closing change {} has an invalid worktree", record.id);
                }
                restore_active(&capsule, &record.id)?;
                continue;
            }

            if self.branch_exists(&record.id)? {
                let live_oid = self.branch_oid(&record.id)?;
                if live_oid != closure.tip_oid {
                    bail!(
                        "closing change '{}' branch changed before recovery",
                        record.id
                    );
                }
                self.validate_target_snapshot(
                    &primary,
                    closure.target_ref.as_deref(),
                    closure.target_oid.as_deref(),
                )?;
                self.delete_branch(&primary, &record.id, &closure.tip_oid)
                    .context("could not finish interrupted branch cleanup")?;
            }
            mark_archived(&capsule, &record.id)
                .context("could not finish interrupted archive record")?;
            finalized += 1;
        }
        Ok(finalized)
    }

    pub(crate) fn prepare_removal(&self, branch: &str, force: bool) -> Result<PreparedRemoval> {
        let worktrees = self.worktrees()?;
        let primary = worktrees
            .first()
            .context("repository has no worktrees")?
            .path
            .clone();
        let current = self.current_root()?;
        let target = worktrees
            .iter()
            .find(|worktree| worktree.branch.as_deref() == Some(branch))
            .with_context(|| format!("branch '{branch}' has no worktree"))?;
        if target.path == primary {
            bail!("cannot remove the primary worktree");
        }
        if target.locked && !force {
            bail!("worktree is locked: {}", target.path.display());
        }
        if !force && self.is_dirty(&target.path)? {
            bail!(
                "worktree has uncommitted changes: {}",
                target.path.display()
            );
        }

        let path = target.path.clone();
        let branch = branch.to_owned();
        let capsule = self.changes_root()?.join(&branch);
        let record_path = capsule.join("change.json");
        let record = Record::load_optional(&record_path)?
            .with_context(|| format!("change record is missing from {}", capsule.display()))?;
        if record.id != branch || !record.state.is_active() {
            bail!("change '{branch}' is not active");
        }
        let expected_oid = self.branch_oid(&branch)?;
        let base = Lineage::load(self, &branch)?.base(self, &branch)?;
        let target_ref = base.removal_ref.clone();
        let target_oid = target_ref
            .as_deref()
            .map(|reference| self.peel_commit(reference))
            .transpose()?;
        let integration = if force {
            "forced".to_owned()
        } else {
            if !self.branch_integrated(&branch, &base)? {
                bail!("branch '{branch}' is not merged; use --force to discard it");
            }
            let target = base
                .removal_ref
                .as_deref()
                .context("the default branch cannot be removed as a linked worktree")?;
            if self.is_ancestor(&branch, target)? {
                "ancestor".to_owned()
            } else if self.same_tree(&branch, target)? {
                "same-tree".to_owned()
            } else {
                "merge-equivalent".to_owned()
            }
        };

        Ok(PreparedRemoval {
            path,
            branch,
            capsule,
            base_oid: record.creation.base_oid,
            primary,
            current,
            expected_oid,
            target_oid,
            target_ref,
            integration,
            force,
        })
    }

    pub(crate) fn remove(&self, prepared: PreparedRemoval) -> Result<Removal> {
        self.archive(&prepared)?;
        mark_closing(
            &prepared.capsule,
            &prepared.branch,
            Closure {
                closed_at: None,
                outcome: if prepared.force {
                    Outcome::Discarded
                } else {
                    Outcome::Integrated
                },
                tip_oid: prepared.expected_oid.clone(),
                target_oid: prepared.target_oid.clone(),
                target_ref: prepared.target_ref.clone(),
                integration: prepared.integration.clone(),
            },
        )?;
        self.validate_removal_refs(&prepared)?;
        self.worktree_remove(&prepared.path, prepared.force)?;
        self.delete_branch(&prepared.primary, &prepared.branch, &prepared.expected_oid)
            .context("worktree was removed, but branch cleanup did not complete")?;
        mark_archived(&prepared.capsule, &prepared.branch)
            .context("change was removed, but its archive record did not close")?;
        Ok(Removal {
            navigate_to: (prepared.path == prepared.current).then_some(prepared.primary),
        })
    }

    fn validate_removal_refs(&self, prepared: &PreparedRemoval) -> Result<()> {
        let live_oid = self.branch_oid(&prepared.branch)?;
        if live_oid != prepared.expected_oid {
            bail!(
                "branch '{}' changed before it could be deleted",
                prepared.branch
            );
        }
        self.validate_target(prepared)
    }

    fn validate_target(&self, prepared: &PreparedRemoval) -> Result<()> {
        self.validate_target_snapshot(
            &prepared.primary,
            prepared.target_ref.as_deref(),
            prepared.target_oid.as_deref(),
        )
    }

    fn validate_target_snapshot(
        &self,
        cwd: &Path,
        target_ref: Option<&str>,
        target_oid: Option<&str>,
    ) -> Result<()> {
        if let (Some(reference), Some(expected)) = (target_ref, target_oid) {
            let live = self.peel_commit_at(cwd, reference)?;
            if live != *expected {
                bail!("integration target '{reference}' changed during removal");
            }
        }
        Ok(())
    }

    fn archive(&self, prepared: &PreparedRemoval) -> Result<()> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before the Unix epoch")?
            .as_nanos();
        let index = prepared
            .capsule
            .join(format!(".archive-index-{}-{nonce}", std::process::id()));
        let temporary = prepared
            .capsule
            .join(format!(".artifacts-{}-{nonce}", std::process::id()));
        create_private_directory(&temporary).with_context(|| {
            format!(
                "failed to create temporary change artifacts {}",
                temporary.display()
            )
        })?;

        let result = (|| {
            self.checked_with_index(&prepared.path, &["read-tree", &prepared.base_oid], &index)?;
            self.checked_with_index(&prepared.path, &["add", "-A", "--", "."], &index)?;
            let final_tree = String::from_utf8_lossy(&self.checked_with_index(
                &prepared.path,
                &["write-tree"],
                &index,
            )?)
            .trim()
            .to_owned();
            let patch = self.checked_at(
                &prepared.path,
                &[
                    "diff",
                    "--binary",
                    "--full-index",
                    "--find-renames",
                    "--no-ext-diff",
                    &prepared.base_oid,
                    &final_tree,
                    "--",
                ],
            )?;
            let statuses = self.checked_at(
                &prepared.path,
                &[
                    "diff",
                    "--name-status",
                    "--find-renames",
                    "-z",
                    &prepared.base_oid,
                    &final_tree,
                    "--",
                ],
            )?;
            let numstat = self.checked_at(
                &prepared.path,
                &[
                    "diff",
                    "--numstat",
                    "--find-renames",
                    "-z",
                    &prepared.base_oid,
                    &final_tree,
                    "--",
                ],
            )?;
            let files = archive_files(&statuses, &numstat)?;
            let summary = summarize_files(&files);
            let stats = ArchiveStats {
                version: 1,
                base_oid: prepared.base_oid.clone(),
                final_tree,
                patch_digest: blake3::hash(&patch).to_hex().to_string(),
                summary,
                files,
            };
            let mut stats_bytes = serde_json::to_vec_pretty(&stats)?;
            stats_bytes.push(b'\n');
            write_private(&temporary.join("change.patch"), &patch)?;
            write_private(&temporary.join("stats.json"), &stats_bytes)?;
            File::open(&temporary)
                .with_context(|| {
                    format!(
                        "failed to open temporary artifact directory {}",
                        temporary.display()
                    )
                })?
                .sync_all()
                .with_context(|| {
                    format!(
                        "failed to sync temporary artifact directory {}",
                        temporary.display()
                    )
                })?;

            let artifacts = prepared.capsule.join("artifacts");
            if artifacts.is_dir() {
                let identical = fs::read(artifacts.join("change.patch")).ok().as_deref()
                    == Some(patch.as_slice())
                    && fs::read(artifacts.join("stats.json")).ok().as_deref()
                        == Some(stats_bytes.as_slice());
                if identical {
                    fs::remove_dir_all(&temporary).with_context(|| {
                        format!(
                            "failed to remove duplicate artifact directory {}",
                            temporary.display()
                        )
                    })?;
                    return Ok(());
                }
                bail!("existing change artifacts differ from the current worktree snapshot");
            }
            fs::rename(&temporary, &artifacts).with_context(|| {
                format!("failed to install change artifacts {}", artifacts.display())
            })?;
            File::open(&prepared.capsule)
                .with_context(|| {
                    format!(
                        "failed to open change capsule {}",
                        prepared.capsule.display()
                    )
                })?
                .sync_all()
                .with_context(|| {
                    format!(
                        "failed to sync change capsule {}",
                        prepared.capsule.display()
                    )
                })?;
            Ok(())
        })();

        let _ = fs::remove_file(&index);
        if result.is_err() {
            let _ = fs::remove_dir_all(&temporary);
        }
        result
    }

    fn branch_exists(&self, branch: &str) -> Result<bool> {
        self.predicate(&[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
    }

    fn default_branch(&self) -> Result<String> {
        if let Ok(remote) = self.text(&[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ]) {
            let local = remote.strip_prefix("origin/").unwrap_or(&remote);
            return if self.branch_exists(local)? {
                Ok(local.to_owned())
            } else {
                Ok(remote)
            };
        }
        for branch in ["main", "master"] {
            if self.branch_exists(branch)? {
                return Ok(branch.to_owned());
            }
        }
        self.worktrees()?
            .first()
            .and_then(|worktree| worktree.branch.as_deref())
            .map(str::to_owned)
            .context("could not detect the default branch")
    }

    fn current_root(&self) -> Result<PathBuf> {
        Ok(PathBuf::from(self.text(&["rev-parse", "--show-toplevel"])?))
    }

    fn common_dir(&self) -> Result<PathBuf> {
        let path = PathBuf::from(self.text(&["rev-parse", "--git-common-dir"])?);
        let path = if path.is_absolute() {
            path
        } else {
            self.cwd.join(path)
        };
        path.canonicalize()
            .context("failed to resolve Git common directory")
    }

    fn changes_root(&self) -> Result<PathBuf> {
        let common_dir = self.common_dir()?;
        let (repositories, name) = self.repository_storage()?;
        locate_repository(&repositories, &name, &common_dir)
    }

    fn repository_storage(&self) -> Result<(PathBuf, String)> {
        let primary = self
            .worktrees()?
            .into_iter()
            .next()
            .context("repository has no worktrees")?;
        let repo = primary
            .path
            .file_name()
            .context("primary worktree has no directory name")?
            .to_string_lossy()
            .into_owned();
        let home = std::env::var_os("HOME").context("HOME is not set")?;
        Ok((PathBuf::from(home).join(".grove/repositories"), repo))
    }

    fn worktrees(&self) -> Result<Vec<Worktree>> {
        let bytes = self.output_bytes(&["worktree", "list", "--porcelain", "-z"])?;
        bytes
            .split(|byte| *byte == 0)
            .collect::<Vec<_>>()
            .split(|field| field.is_empty())
            .filter(|record| !record.is_empty())
            .map(|record| {
                let mut path = None;
                let mut branch = None;
                let mut locked = false;
                let mut prunable = false;
                for field in record {
                    if let Some(value) = field.strip_prefix(b"worktree ") {
                        path = Some(path_from_bytes(value)?);
                    } else if let Some(value) = field.strip_prefix(b"branch refs/heads/") {
                        branch = Some(
                            String::from_utf8(value.to_vec())
                                .context("Git returned a non-UTF-8 branch name")?,
                        );
                    } else if *field == b"locked" || field.starts_with(b"locked ") {
                        locked = true;
                    } else if *field == b"prunable" || field.starts_with(b"prunable ") {
                        prunable = true;
                    }
                }
                Ok(Worktree {
                    path: path.context("Git returned a worktree without a path")?,
                    branch,
                    locked,
                    prunable,
                })
            })
            .collect()
    }

    fn is_dirty(&self, path: &Path) -> Result<bool> {
        Ok(!self
            .text_at(path, &["status", "--porcelain", "--untracked-files=normal"])?
            .is_empty())
    }

    fn status(&self, path: &Path) -> Result<Status> {
        let porcelain = self.text_at(path, &["status", "--porcelain"])?;
        let mut conflicts = 0;
        for line in porcelain.lines() {
            let code = line.as_bytes().get(..2).unwrap_or_default();
            if matches!(code, b"DD" | b"AU" | b"UD" | b"UA" | b"DU" | b"AA" | b"UU") {
                conflicts += 1;
            }
        }
        let mut added = 0;
        let mut deleted = 0;
        for line in self.text_at(path, &["diff", "--numstat", "HEAD"])?.lines() {
            let mut fields = line.split('\t');
            added += fields
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            deleted += fields
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        }
        let untracked =
            self.output_bytes_at(path, &["ls-files", "--others", "--exclude-standard", "-z"])?;
        for relative in untracked
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
        {
            let contents = std::fs::read(path.join(path_from_bytes(relative)?))?;
            if !contents.contains(&0) {
                added += contents.iter().filter(|byte| **byte == b'\n').count();
                if !contents.is_empty() && !contents.ends_with(b"\n") {
                    added += 1;
                }
            }
        }
        Ok(Status {
            changed: !porcelain.is_empty(),
            added,
            deleted,
            conflicts,
        })
    }

    fn divergence(&self, path: &Path, base: &str) -> Result<Divergence> {
        let counts = self.text_at(
            path,
            &[
                "rev-list",
                "--left-right",
                "--count",
                &format!("{base}...HEAD"),
            ],
        )?;
        let mut fields = counts.split_whitespace();
        let behind = fields
            .next()
            .context("Git did not return a behind count")?
            .parse()?;
        let ahead = fields
            .next()
            .context("Git did not return an ahead count")?
            .parse()?;
        Ok(Divergence { ahead, behind })
    }

    fn branch_integrated(&self, branch: &str, base: &BranchBase) -> Result<bool> {
        if !base.valid {
            bail!("branch '{branch}' has invalid Grove lineage; use --force to discard it");
        }
        let comparison = base
            .removal_ref
            .as_deref()
            .context("the default branch cannot be removed as a linked worktree")?;
        if self.is_ancestor(branch, comparison)? || self.same_tree(branch, comparison)? {
            return Ok(true);
        }
        self.merge_adds_no_change(branch, comparison)
    }

    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        self.predicate(&["merge-base", "--is-ancestor", ancestor, descendant])
    }

    fn branch_oid(&self, branch: &str) -> Result<String> {
        self.text(&["rev-parse", &format!("refs/heads/{branch}")])
    }

    fn worktree_add(&self, path: &Path, branch: &str) -> Result<()> {
        self.output_os(&["worktree", "add"], path, &[branch])
    }

    fn reserve_branch(&self, branch: &str, oid: &str) -> Result<()> {
        let reference = format!("refs/heads/{branch}");
        let output = self.raw(&["update-ref", &reference, oid, ""])?;
        check(output, &["update-ref", "<branch>", "<oid>", "<missing>"]).map(|_| ())
    }

    fn worktree_remove(&self, path: &Path, force: bool) -> Result<()> {
        let before = if force {
            &["worktree", "remove", "--force", "--force"][..]
        } else {
            &["worktree", "remove"][..]
        };
        self.output_os(before, path, &[])
    }

    fn delete_branch(&self, cwd: &Path, branch: &str, expected: &str) -> Result<()> {
        let reference = format!("refs/heads/{branch}");
        let mut command = Command::new("git");
        command.arg("-C").arg(cwd);
        command.args(["update-ref", "-d", &reference, expected]);
        let output = command.output().context("could not delete branch")?;
        check(output, &["update-ref", "-d", "<branch>", "<expected>"])
            .with_context(|| format!("branch '{branch}' changed before it could be deleted"))?;
        Ok(())
    }

    fn text(&self, args: &[&str]) -> Result<String> {
        self.text_at(&self.cwd, args)
    }

    fn peel_commit(&self, source: &str) -> Result<String> {
        self.peel_commit_at(&self.cwd, source)
    }

    fn peel_commit_at(&self, cwd: &Path, source: &str) -> Result<String> {
        let revision = format!("{source}^{{commit}}");
        let args = [
            "rev-parse",
            "--verify",
            "--end-of-options",
            revision.as_str(),
        ];
        let output = self.raw_at(cwd, &args)?;
        if !output.status.success() {
            bail!("base '{source}' does not resolve to a commit");
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn local_branch(&self, source: &str) -> Result<Option<String>> {
        let output = self.raw(&[
            "rev-parse",
            "--symbolic-full-name",
            "--verify",
            "--end-of-options",
            source,
        ])?;
        if !output.status.success() {
            return Ok(None);
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .strip_prefix("refs/heads/")
            .filter(|branch| !branch.is_empty())
            .map(str::to_owned))
    }

    fn normalized_default(&self) -> Result<(String, String)> {
        let reference = self.default_branch()?;
        let name = reference
            .strip_prefix("origin/")
            .unwrap_or(&reference)
            .to_owned();
        Ok((name, reference))
    }

    fn is_full_commit(&self, oid: &str) -> bool {
        self.peel_commit(oid).is_ok_and(|resolved| resolved == oid)
    }

    fn same_tree(&self, branch: &str, base: &str) -> Result<bool> {
        self.predicate(&["diff", "--quiet", branch, base])
    }

    fn merge_adds_no_change(&self, branch: &str, base: &str) -> Result<bool> {
        let output = self.raw(&["merge-tree", "--write-tree", base, branch])?;
        if !output.status.success() {
            return match output.status.code() {
                Some(1) => Ok(false),
                _ => {
                    check(
                        output,
                        &["merge-tree", "--write-tree", "<base>", "<branch>"],
                    )?;
                    unreachable!()
                }
            };
        }
        let merged_tree = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .context("git merge-tree did not return a tree")?
            .to_owned();
        Ok(merged_tree == self.text(&["rev-parse", &format!("{base}^{{tree}}")])?)
    }

    fn rollback_created_worktree(&self, path: &Path, branch: &str) -> Result<()> {
        if self
            .worktrees()?
            .iter()
            .any(|worktree| worktree.branch.as_deref() == Some(branch))
        {
            self.worktree_remove(path, true)
                .context("failed to roll back created worktree")?;
        }
        if self.branch_exists(branch)? {
            let expected = self.branch_oid(branch)?;
            let reference = format!("refs/heads/{branch}");
            let output = self.raw(&["update-ref", "-d", &reference, &expected])?;
            check(output, &["update-ref", "-d", "<branch>", "<expected>"])
                .context("failed to roll back created branch")?;
        }
        Ok(())
    }

    fn text_at(&self, cwd: &Path, args: &[&str]) -> Result<String> {
        self.checked_at(cwd, args)
            .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_owned())
    }

    fn output_bytes(&self, args: &[&str]) -> Result<Vec<u8>> {
        self.output_bytes_at(&self.cwd, args)
    }

    fn output_bytes_at(&self, cwd: &Path, args: &[&str]) -> Result<Vec<u8>> {
        self.checked_at(cwd, args)
    }

    fn checked_at(&self, cwd: &Path, args: &[&str]) -> Result<Vec<u8>> {
        check(self.raw_at(cwd, args)?, args)
    }

    fn checked_with_index(&self, cwd: &Path, args: &[&str], index: &Path) -> Result<Vec<u8>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .env("GIT_INDEX_FILE", index)
            .output()
            .with_context(|| format!("could not run git {}", args.join(" ")))?;
        check(output, args)
    }

    fn raw(&self, args: &[&str]) -> Result<Output> {
        self.raw_at(&self.cwd, args)
    }

    fn predicate(&self, args: &[&str]) -> Result<bool> {
        let output = self.raw(args)?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => {
                check(output, args)?;
                unreachable!()
            }
        }
    }

    fn raw_at(&self, cwd: &Path, args: &[&str]) -> Result<Output> {
        Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .output()
            .with_context(|| format!("could not run git {}", args.join(" ")))
    }

    fn output_os(&self, before: &[&str], path: &Path, after: &[&str]) -> Result<()> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.cwd)
            .args(before)
            .arg(path)
            .args(after)
            .output()
            .context("could not run git worktree")?;
        let mut shown = before.to_vec();
        shown.push("<path>");
        shown.extend_from_slice(after);
        check(output, &shown).map(|_| ())
    }
}

fn abbreviate_oid(oid: &str) -> String {
    oid.chars().take(12).collect()
}

fn archive_files(statuses: &[u8], numstat: &[u8]) -> Result<Vec<ArchiveFile>> {
    let numbers = parse_numstat(numstat)?;
    let fields = statuses
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let status = String::from_utf8_lossy(fields[index]).into_owned();
        index += 1;
        let old_path = if status.starts_with('R') || status.starts_with('C') {
            let path = fields
                .get(index)
                .context("Git returned an incomplete rename")?;
            index += 1;
            Some(String::from_utf8_lossy(path).into_owned())
        } else {
            None
        };
        let path = fields
            .get(index)
            .context("Git returned a status without a path")?;
        index += 1;
        let path = String::from_utf8_lossy(path).into_owned();
        let (additions, deletions) = numbers.get(&path).cloned().unwrap_or((Some(0), Some(0)));
        files.push(ArchiveFile {
            path,
            old_path,
            status,
            binary: additions.is_none() || deletions.is_none(),
            additions,
            deletions,
        });
    }
    Ok(files)
}

type LineCounts = (Option<u64>, Option<u64>);

fn parse_numstat(bytes: &[u8]) -> Result<HashMap<String, LineCounts>> {
    let fields = bytes.split(|byte| *byte == 0).collect::<Vec<_>>();
    let mut counts = HashMap::new();
    let mut index = 0;
    while index < fields.len() {
        let field = fields[index];
        index += 1;
        if field.is_empty() {
            continue;
        }
        let mut pieces = field.splitn(3, |byte| *byte == b'\t');
        let additions = parse_line_count(pieces.next().context("Git numstat omitted additions")?)?;
        let deletions = parse_line_count(pieces.next().context("Git numstat omitted deletions")?)?;
        let embedded_path = pieces.next().context("Git numstat omitted path")?;
        let path = if embedded_path.is_empty() {
            index += 1;
            let new_path = fields
                .get(index)
                .context("Git numstat omitted rename target")?;
            index += 1;
            *new_path
        } else {
            embedded_path
        };
        counts.insert(
            String::from_utf8_lossy(path).into_owned(),
            (additions, deletions),
        );
    }
    Ok(counts)
}

fn parse_line_count(value: &[u8]) -> Result<Option<u64>> {
    if value == b"-" {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(value)
            .parse()
            .context("Git returned an invalid numstat count")?,
    ))
}

fn summarize_files(files: &[ArchiveFile]) -> FileSummary {
    let mut summary = FileSummary {
        files_changed: files.len(),
        ..FileSummary::default()
    };
    for file in files {
        summary.additions += file.additions.unwrap_or(0);
        summary.deletions += file.deletions.unwrap_or(0);
        if file.binary {
            summary.binary += 1;
        }
        match file.status.as_bytes().first().copied() {
            Some(b'A') => summary.added += 1,
            Some(b'D') => summary.deleted += 1,
            Some(b'R') => summary.renamed += 1,
            Some(b'C') => summary.copied += 1,
            _ => summary.modified += 1,
        }
    }
    summary
}

fn create_private_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(path)
}

fn write_private(path: &Path, contents: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(path)
        .with_context(|| format!("failed to create archive artifact {}", path.display()))?;
    file.write_all(contents)
        .with_context(|| format!("failed to write archive artifact {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync archive artifact {}", path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf> {
    Ok(PathBuf::from(OsString::from_vec(bytes.to_vec())))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf> {
    Ok(PathBuf::from(
        String::from_utf8(bytes.to_vec()).context("Git returned a non-UTF-8 worktree path")?,
    ))
}

fn check(output: Output, args: &[&str]) -> Result<Vec<u8>> {
    if output.status.success() {
        return Ok(output.stdout);
    }
    let message = String::from_utf8_lossy(&output.stderr);
    bail!("git {} failed: {}", args.join(" "), message.trim())
}
