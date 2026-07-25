use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[cfg(unix)]
use std::{ffi::OsString, os::unix::ffi::OsStringExt};

use crate::change::{
    Change, Closing, Creation, Outcome, Record, RepositoryDirectory, lock as lock_change,
    mark_archived, mark_closing, restore_active, try_lock as try_lock_change,
};
use anyhow::{Context, Result, bail};

#[derive(Clone)]
pub(crate) struct Git {
    cwd: PathBuf,
}

#[derive(Debug)]
struct Worktree {
    path: PathBuf,
    head_oid: String,
    branch: Option<String>,
    locked: bool,
    prunable: bool,
}

pub(crate) struct Status {
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

pub(crate) struct CurrentChange {
    pub(crate) path: PathBuf,
    pub(crate) title: Option<String>,
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

pub(crate) struct SyncEntry {
    pub(crate) id: String,
    pub(crate) title: Option<String>,
    pub(crate) outcome: String,
    pub(crate) reason: String,
}

pub(crate) struct SyncResult {
    pub(crate) entries: Vec<SyncEntry>,
    pub(crate) archived: usize,
    pub(crate) rebased: usize,
    pub(crate) skipped: usize,
}

pub(crate) struct PushRemote {
    pub(crate) name: String,
    pub(crate) url: String,
}

pub(crate) struct ShipSnapshot {
    pub(crate) staged: bool,
    pub(crate) summary: String,
    head_oid: String,
    index_tree: String,
    worktree_state: Vec<u8>,
}

pub(crate) struct PreparedArchive {
    path: PathBuf,
    id: String,
    capsule: PathBuf,
    primary: PathBuf,
    expected_head_oid: String,
    target_oid: Option<String>,
    target_ref: Option<String>,
    local_branch: Option<String>,
    force: bool,
}

struct BranchBase {
    display: String,
    divergence_ref: Option<String>,
    removal_ref: Option<String>,
    valid: bool,
}

struct Lineage {
    base_oid: String,
    parent: Option<String>,
}

impl Lineage {
    fn resolve_creation_base(git: &Git, source: Option<&str>) -> Result<Creation> {
        let (source, default_base) = match source {
            Some(source) => (source.to_owned(), false),
            None => (git.default_branch()?, true),
        };
        let base_oid = if default_base {
            git.peel_commit(&source).with_context(|| {
                format!(
                    "cannot create a Change from default base '{source}': create an initial commit or pass --from a commit"
                )
            })?
        } else {
            git.peel_commit(&source)?
        };
        let parent = if source == "@" {
            git.text(&["symbolic-ref", "--quiet", "--short", "HEAD"])
                .ok()
        } else {
            git.local_branch(&source)?
        };
        Ok(Creation { base_oid, parent })
    }

    fn from_record(record: &Record) -> Self {
        Self {
            base_oid: record.base_oid.clone(),
            parent: record.parent.clone(),
        }
    }

    fn base(&self, git: &Git) -> Result<BranchBase> {
        if !git.is_full_commit(&self.base_oid) || self.parent.as_deref().is_some_and(str::is_empty)
        {
            return Ok(BranchBase {
                display: "invalid lineage".to_owned(),
                divergence_ref: None,
                removal_ref: None,
                valid: false,
            });
        }
        if let Some(parent) = &self.parent
            && git.branch_exists(parent)?
            && git.is_ancestor(&self.base_oid, parent)?
        {
            return Ok(BranchBase {
                display: parent.clone(),
                divergence_ref: Some(parent.clone()),
                removal_ref: Some(parent.clone()),
                valid: true,
            });
        }
        Ok(BranchBase {
            display: abbreviate_oid(&self.base_oid),
            divergence_ref: Some(self.base_oid.clone()),
            removal_ref: Some(self.base_oid.clone()),
            valid: true,
        })
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
        let reserved = self.repository()?.reserve(creation.clone())?;
        let path = reserved.workspace();
        if let Err(error) = self.worktree_add_detached(&path, &creation.base_oid) {
            let git_rollback = self.rollback_created_worktree(&path);
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
        Ok(reserved.finish())
    }

    pub(crate) fn inventory(&self) -> Result<Vec<WorktreeView>> {
        let worktrees = self.worktrees()?;
        let current = self.current_root()?;
        let repository = self.repository()?;
        let mut records = Vec::new();
        for (capsule, record) in repository.records()? {
            if !record.state.is_active() {
                continue;
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
                let expected_path = capsule.join("workspace");
                let worktree = managed_worktree(&worktrees, &expected_path).with_context(|| {
                    format!("active Change {} has no expected worktree", record.id)
                })?;

                let base = Lineage::from_record(&record).base(self)?;
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

    pub(crate) fn current_change(&self) -> Result<Option<CurrentChange>> {
        let current = self.current_root()?;
        let repository = self.repository()?;
        for (capsule, record) in repository.records()? {
            if record.state.is_active() && capsule.join("workspace") == current {
                let worktrees = self.worktrees()?;
                let worktree = managed_worktree(&worktrees, &current)
                    .context("current Change has no expected worktree")?;
                if worktree.prunable {
                    bail!("current Change worktree is missing");
                }
                return Ok(Some(CurrentChange {
                    path: current,
                    title: record.title,
                }));
            }
        }
        Ok(None)
    }

    pub(crate) fn push_remote(&self, path: &Path) -> Result<PushRemote> {
        self.validate_resolved_state(path)?;
        let remotes = self.text_at(path, &["remote"])?;
        let remotes = remotes.lines().collect::<Vec<_>>();
        let branch = self
            .symbolic_head_at(path)?
            .and_then(|head| head.strip_prefix("refs/heads/").map(str::to_owned));
        let branch_push_remote = branch
            .as_deref()
            .map(|branch| self.config_optional_at(path, &format!("branch.{branch}.pushRemote")))
            .transpose()?
            .flatten();
        let default_push_remote = self.config_optional_at(path, "remote.pushDefault")?;
        let branch_remote = branch
            .as_deref()
            .map(|branch| self.config_optional_at(path, &format!("branch.{branch}.remote")))
            .transpose()?
            .flatten();
        let configured = branch_push_remote.or(default_push_remote).or(branch_remote);
        if configured.as_deref() == Some(".") {
            bail!("cannot ship without a network push remote");
        }
        let remote = configured
            .or_else(|| remotes.contains(&"origin").then(|| "origin".to_owned()))
            .or_else(|| (remotes.len() == 1).then(|| remotes[0].to_owned()))
            .context("cannot ship without a usable push remote")?;
        if !remotes.contains(&remote.as_str()) {
            bail!("configured push remote '{remote}' does not exist");
        }
        if !remote.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        }) {
            bail!("push remote has an unsupported name");
        }
        let url = self
            .text_at(path, &["remote", "get-url", "--push", &remote])
            .with_context(|| format!("push remote '{remote}' has no usable URL"))?;
        Ok(PushRemote {
            name: remote,
            url: network_remote(&url)?,
        })
    }

    pub(crate) fn fetch_ship_base(
        &self,
        path: &Path,
        remote: &str,
        branch: &str,
    ) -> Result<String> {
        let tracking = format!("refs/remotes/{remote}/{branch}");
        let refspec = format!("+refs/heads/{branch}:{tracking}");
        self.checked_at(
            path,
            &[
                "fetch",
                "--quiet",
                "--no-tags",
                "--no-prune",
                "--no-recurse-submodules",
                remote,
                &refspec,
            ],
        )?;
        Ok(tracking)
    }

    pub(crate) fn remote_branch_exists(
        &self,
        path: &Path,
        remote: &str,
        branch: &str,
    ) -> Result<bool> {
        let args = ["ls-remote", "--exit-code", "--heads", remote, branch];
        let output = self.raw_at(path, &args)?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(2) => Ok(false),
            _ => {
                check(output, &args)?;
                unreachable!()
            }
        }
    }

    pub(crate) fn has_publishable_work(&self, path: &Path, base: &str) -> Result<bool> {
        if self.is_dirty(path)? {
            return Ok(true);
        }
        let count = self.text_at(path, &["rev-list", "--count", &format!("{base}..HEAD")])?;
        Ok(count != "0")
    }

    pub(crate) fn prepare_ship(&self, path: &Path, branch: &str) -> Result<()> {
        self.validate_resolved_state(path)?;
        match self
            .symbolic_head_at(path)?
            .and_then(|head| head.strip_prefix("refs/heads/").map(str::to_owned))
        {
            Some(current) if current != branch => {
                bail!("Change is already on publication branch '{current}', expected '{branch}'")
            }
            Some(_) => {}
            None if self.branch_exists_at(path, branch)? => {
                bail!("publication branch '{branch}' already belongs to another Change")
            }
            None => {
                self.checked_at(path, &["switch", "--create", branch])?;
            }
        }
        self.checked_at(path, &["add", "--all"])?;
        Ok(())
    }

    pub(crate) fn capture_ship_snapshot(&self, path: &Path, base: &str) -> Result<ShipSnapshot> {
        let staged = self.differs_at(path, &["diff", "--cached", "--quiet", "--exit-code"])?;
        let head_oid = self.text_at(path, &["rev-parse", "HEAD"])?;
        let index_tree = self.text_at(path, &["write-tree"])?;
        let worktree_state = self.output_bytes_at(
            path,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        )?;
        let files = self.text_at(path, &["diff", "--name-status", base])?;
        let statistics = self.text_at(path, &["diff", "--stat", base])?;
        let incremental_files = self.text_at(path, &["diff", "--cached", "--name-status"])?;
        let incremental_statistics = self.text_at(path, &["diff", "--cached", "--stat"])?;
        let subjects = self.text_at(path, &["log", "--format=%s", &format!("{base}..HEAD")])?;
        let patch = self.checked_at(path, &["diff", "--cached", "--unified=0"])?;
        let truncated = patch.len() > 32 * 1024;
        let patch = String::from_utf8_lossy(&patch[..patch.len().min(32 * 1024)]);
        let truncation = if truncated {
            "\nThe detailed patch is truncated. Use read selectively if the intent remains unclear."
        } else {
            ""
        };
        Ok(ShipSnapshot {
            staged,
            head_oid,
            index_tree,
            worktree_state,
            summary: format!(
                "Complete changed files:\n{files}\n\nComplete diff statistics:\n{statistics}\n\nExisting commit subjects:\n{subjects}\n\nNewly staged files:\n{incremental_files}\n\nNewly staged statistics:\n{incremental_statistics}\n\nNewly staged zero-context patch:\n{patch}{truncation}"
            ),
        })
    }

    pub(crate) fn validate_ship_snapshot(
        &self,
        path: &Path,
        branch: &str,
        context: &ShipSnapshot,
    ) -> Result<()> {
        let current = self
            .symbolic_head_at(path)?
            .and_then(|head| head.strip_prefix("refs/heads/").map(str::to_owned));
        if current.as_deref() != Some(branch)
            || self.text_at(path, &["rev-parse", "HEAD"])? != context.head_oid
            || self.text_at(path, &["write-tree"])? != context.index_tree
            || self.output_bytes_at(
                path,
                &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
            )? != context.worktree_state
        {
            bail!("Change Git state changed while shipping; rerun grove ship");
        }
        Ok(())
    }

    pub(crate) fn commit_ship(
        &self,
        path: &Path,
        subject: &str,
        context: &ShipSnapshot,
    ) -> Result<()> {
        self.checked_at(path, &["commit", "--quiet", "--message", subject])?;
        if self.text_at(path, &["rev-parse", "HEAD^"])? != context.head_oid
            || self.text_at(path, &["rev-parse", "HEAD^{tree}"])? != context.index_tree
        {
            bail!("Change Git state changed while committing; refusing to push");
        }
        Ok(())
    }

    pub(crate) fn validate_clean_ship(&self, path: &Path) -> Result<()> {
        if self.is_dirty(path)? {
            bail!("Change Git state changed while shipping; rerun grove ship");
        }
        Ok(())
    }

    pub(crate) fn push_ship(&self, path: &Path, remote: &str, branch: &str) -> Result<()> {
        let head_oid = self.text_at(path, &["rev-parse", "HEAD"])?;
        let destination = format!("{head_oid}:refs/heads/{branch}");
        self.checked_at(path, &["push", remote, &destination])?;
        self.checked_at(
            path,
            &["config", &format!("branch.{branch}.remote"), remote],
        )?;
        self.checked_at(
            path,
            &[
                "config",
                &format!("branch.{branch}.merge"),
                &format!("refs/heads/{branch}"),
            ],
        )?;
        Ok(())
    }

    fn differs_at(&self, path: &Path, args: &[&str]) -> Result<bool> {
        let output = self.raw_at(path, args)?;
        match output.status.code() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => {
                check(output, args)?;
                unreachable!()
            }
        }
    }

    fn validate_resolved_state(&self, path: &Path) -> Result<()> {
        if self.status(path)?.conflicts > 0 {
            bail!("cannot ship a Change with unresolved conflicts");
        }
        for marker in [
            "MERGE_HEAD",
            "CHERRY_PICK_HEAD",
            "REVERT_HEAD",
            "rebase-merge",
            "rebase-apply",
        ] {
            let marker = PathBuf::from(self.text_at(path, &["rev-parse", "--git-path", marker])?);
            let marker = if marker.is_absolute() {
                marker
            } else {
                path.join(marker)
            };
            if marker.exists() {
                bail!("cannot ship while a Git operation is in progress");
            }
        }
        Ok(())
    }

    pub(crate) fn sync(&self) -> Result<SyncResult> {
        let worktrees = self.worktrees()?;
        let primary = worktrees.first().context("repository has no worktrees")?;
        if self.current_root()? != primary.path {
            bail!("grove sync must be run from the primary worktree");
        }
        let primary_branch = primary
            .branch
            .as_deref()
            .context("primary worktree is not on a branch")?;
        let upstream = self
            .text_at(
                &primary.path,
                &[
                    "rev-parse",
                    "--symbolic-full-name",
                    &format!("{primary_branch}@{{upstream}}"),
                ],
            )
            .with_context(|| format!("primary branch '{primary_branch}' has no upstream"))?;
        let remote = self
            .text_at(
                &primary.path,
                &[
                    "config",
                    "--get",
                    &format!("branch.{primary_branch}.remote"),
                ],
            )
            .with_context(|| format!("primary branch '{primary_branch}' has no remote"))?;
        let merge_ref = self
            .text_at(
                &primary.path,
                &["config", "--get", &format!("branch.{primary_branch}.merge")],
            )
            .with_context(|| format!("primary branch '{primary_branch}' has no merge ref"))?;
        if self.is_dirty(&primary.path)? {
            bail!("primary worktree has uncommitted changes");
        }

        let repository = self.repository()?;
        let mut records = repository
            .records()?
            .into_iter()
            .filter(|(_, record)| record.state.is_active())
            .map(|(capsule, record)| (record.created_at, capsule, record))
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.2.id.cmp(&right.2.id))
        });

        let upstream_refspec = format!("+{merge_ref}:{upstream}");
        self.checked_at(
            &primary.path,
            &[
                "fetch",
                "--quiet",
                "--no-tags",
                "--no-prune",
                "--no-recurse-submodules",
                &remote,
                &upstream_refspec,
            ],
        )
        .with_context(|| {
            format!(
                "failed to fetch merge ref '{merge_ref}' from remote '{remote}' into '{upstream}'"
            )
        })?;
        let upstream_oid = self
            .peel_commit_at(&primary.path, &upstream)
            .with_context(|| format!("fetched upstream '{upstream}' is not a commit"))?;
        let primary_ref = format!("refs/heads/{primary_branch}");
        if self.symbolic_head_at(&primary.path)?.as_deref() != Some(&primary_ref)
            || self.peel_commit_at(&primary.path, "HEAD")? != primary.head_oid
        {
            bail!("primary branch changed while fetching upstream");
        }
        if self.is_dirty(&primary.path)? {
            bail!("primary worktree changed while fetching upstream");
        }
        if !self.is_ancestor(&primary.head_oid, &upstream_oid)? {
            bail!("primary branch '{primary_branch}' cannot be fast-forwarded to '{upstream}'");
        }
        self.checked_at(
            &primary.path,
            &[
                "merge",
                "--quiet",
                "--ff-only",
                "--no-autostash",
                &upstream_oid,
            ],
        )
        .with_context(|| {
            format!("failed to fast-forward primary branch '{primary_branch}' to '{upstream}'")
        })?;
        if self.peel_commit_at(&primary.path, "HEAD")? != upstream_oid {
            bail!("primary branch changed while fast-forwarding to '{upstream}'");
        }

        let identities = records
            .iter()
            .map(|(_, _, record)| (record.id.clone(), record.title.clone()))
            .collect::<Vec<_>>();
        let mut outcomes = HashMap::new();
        let mut locks = Vec::new();
        let mut candidates = Vec::new();
        for (_, capsule, record) in records {
            if record.parent.as_deref() != Some(primary_branch) {
                outcomes.insert(record.id, ("skipped", "created from another parent branch"));
                continue;
            }
            let expected_path = capsule.join("workspace");
            let Some(worktree) = managed_worktree(&worktrees, &expected_path) else {
                let reason = unavailable_worktree_reason(&worktrees, &expected_path);
                outcomes.insert(record.id, ("skipped", reason));
                continue;
            };
            if worktree.prunable || !worktree.path.exists() {
                outcomes.insert(record.id, ("skipped", "worktree is missing"));
                continue;
            }
            if worktree.locked {
                outcomes.insert(record.id, ("skipped", "worktree is Git-locked"));
                continue;
            }
            let Some(activity_lock) = try_lock_change(&capsule)? else {
                outcomes.insert(record.id, ("skipped", "Change is already open"));
                continue;
            };
            locks.push(activity_lock);
            candidates.push((record, expected_path));
        }

        let refreshed = self.worktrees()?;
        let mut changes = Vec::new();
        for (record, expected_path) in candidates {
            let Some(worktree) = managed_worktree(&refreshed, &expected_path) else {
                let reason = unavailable_worktree_reason(&refreshed, &expected_path);
                outcomes.insert(record.id, ("skipped", reason));
                continue;
            };
            if worktree.prunable || !worktree.path.exists() {
                outcomes.insert(record.id, ("skipped", "worktree became missing"));
                continue;
            }
            if worktree.locked {
                outcomes.insert(record.id, ("skipped", "worktree became Git-locked"));
                continue;
            }
            if self.peel_commit_at(&worktree.path, "HEAD")? != worktree.head_oid {
                bail!(
                    "Change '{}' HEAD changed during sync preparation",
                    record.id
                );
            }
            if self.is_dirty(&worktree.path)? {
                outcomes.insert(record.id, ("skipped", "worktree has uncommitted changes"));
                continue;
            }
            changes.push((
                record.id,
                worktree.path.clone(),
                worktree.head_oid.clone(),
                record.base_oid,
            ));
        }

        let mut integrated = Vec::new();
        let mut remaining = Vec::new();
        for (id, path, head_oid, creation_base_oid) in changes {
            if !self.is_full_commit(&creation_base_oid) {
                outcomes.insert(id, ("skipped", "recorded creation base is invalid"));
                continue;
            }
            if !self
                .is_ancestor(&creation_base_oid, &upstream_oid)
                .with_context(|| {
                    format!("failed to validate recorded creation base OID for change '{id}'")
                })?
            {
                outcomes.insert(id, ("skipped", "creation base is not in upstream"));
                continue;
            }
            if !self
                .is_ancestor(&creation_base_oid, &head_oid)
                .with_context(|| {
                    format!("failed to validate recorded creation base topology for Change '{id}'")
                })?
            {
                outcomes.insert(
                    id,
                    ("skipped", "Change does not descend from creation base"),
                );
                continue;
            }
            if self.has_merge_history(&creation_base_oid, &head_oid)? {
                outcomes.insert(id, ("skipped", "Change has merge history"));
                continue;
            }
            if let Some(prepared) =
                self.prepare_sync_archive(&id, &path, &head_oid, &upstream, &upstream_oid)?
            {
                integrated.push((id, prepared));
            } else {
                remaining.push((id, path, creation_base_oid));
            }
        }

        for (id, prepared) in integrated {
            self.finish_archive(prepared)?;
            outcomes.insert(id, ("archived", "integrated upstream"));
        }
        for (id, path, creation_base_oid) in &remaining {
            if self.rebase_change(path, &upstream_oid, creation_base_oid)? {
                outcomes.insert(id.clone(), ("rebased", "rebased onto upstream"));
            } else {
                outcomes.insert(id.clone(), ("skipped", "rebase failed; Change restored"));
            }
        }
        drop(locks);

        let entries = identities
            .into_iter()
            .map(|(id, title)| {
                let (outcome, reason) = outcomes
                    .remove(&id)
                    .expect("every active Change has a sync outcome");
                SyncEntry {
                    id,
                    title,
                    outcome: outcome.to_owned(),
                    reason: reason.to_owned(),
                }
            })
            .collect::<Vec<_>>();
        let archived = entries
            .iter()
            .filter(|entry| entry.outcome == "archived")
            .count();
        let rebased = entries
            .iter()
            .filter(|entry| entry.outcome == "rebased")
            .count();
        let skipped = entries
            .iter()
            .filter(|entry| entry.outcome == "skipped")
            .count();

        Ok(SyncResult {
            entries,
            archived,
            rebased,
            skipped,
        })
    }

    fn has_merge_history(&self, base_oid: &str, tip: &str) -> Result<bool> {
        let range = format!("{base_oid}..{tip}");
        Ok(!self
            .output_bytes(&["rev-list", "--min-parents=2", "--max-count=1", &range])?
            .is_empty())
    }

    fn rebase_change(&self, path: &Path, upstream_oid: &str, base_oid: &str) -> Result<bool> {
        let original_ref = self.symbolic_head_at(path)?;
        let original_head = self.peel_commit_at(path, "HEAD")?;
        let original_branch_oid = original_ref
            .as_deref()
            .map(|reference| self.peel_commit_at(path, reference))
            .transpose()?;
        let original_status = self.output_bytes_at(
            path,
            &["status", "--porcelain=v1", "--untracked-files=normal"],
        )?;

        let output = self.raw_at(
            path,
            &[
                "-c",
                "rebase.updateRefs=false",
                "-c",
                "rebase.autoStash=false",
                "rebase",
                "--quiet",
                "--no-autostash",
                "--reapply-cherry-picks",
                "--onto",
                upstream_oid,
                base_oid,
            ],
        )?;
        if output.status.success() {
            return Ok(true);
        }

        self.checked_at(path, &["rebase", "--abort"])
            .with_context(|| format!("failed to abort rebase at {}", path.display()))?;
        let restored = self.symbolic_head_at(path)? == original_ref
            && self.peel_commit_at(path, "HEAD")? == original_head
            && match (&original_ref, &original_branch_oid) {
                (Some(reference), Some(oid)) => self.peel_commit_at(path, reference)? == *oid,
                (None, None) => true,
                _ => false,
            }
            && self.output_bytes_at(
                path,
                &["status", "--porcelain=v1", "--untracked-files=normal"],
            )? == original_status;
        if !restored {
            bail!(
                "rebase abort did not exactly restore change at {}",
                path.display()
            );
        }
        Ok(false)
    }

    pub(crate) fn recover_closing_archives(&self) -> Result<usize> {
        let worktrees = self.worktrees()?;
        let primary = worktrees
            .first()
            .context("repository has no worktrees")?
            .path
            .clone();
        let repository = self.repository()?;
        let mut finalized = 0;
        for (capsule, record) in repository.records()? {
            if !record.state.is_closing() {
                continue;
            }
            let _lock = lock_change(&capsule)?;
            let Some((capsule, record)) = repository.record(&record.id)? else {
                continue;
            };
            if !record.state.is_closing() {
                continue;
            }
            let closing = record
                .closing
                .context("closing Change has no closing facts")?;
            let expected_path = capsule.join("workspace");
            let current_worktrees = self.worktrees()?;
            if managed_worktree(&current_worktrees, &expected_path)
                .is_some_and(|worktree| !worktree.prunable && worktree.path.exists())
            {
                restore_active(&capsule, &record.id)?;
                continue;
            }

            self.validate_target_snapshot(
                &primary,
                closing.target_ref.as_deref(),
                closing.target_oid.as_deref(),
            )?;
            if let Some(branch) = &closing.local_branch {
                self.cleanup_local_branch(&primary, branch, &closing.tip_oid)?;
            }
            mark_archived(&capsule, &record.id)
                .context("could not finish interrupted archive record")?;
            finalized += 1;
        }
        Ok(finalized)
    }

    pub(crate) fn prepare_archive(&self, id: &str, force: bool) -> Result<PreparedArchive> {
        let worktrees = self.worktrees()?;
        let primary = worktrees
            .first()
            .context("repository has no worktrees")?
            .path
            .clone();
        let (capsule, record) = self
            .repository()?
            .record(id)?
            .with_context(|| format!("Change record is missing for '{id}'"))?;
        if record.id != id || !record.state.is_active() {
            bail!("Change '{id}' is not active");
        }
        let expected_path = capsule.join("workspace");
        let target = managed_worktree(&worktrees, &expected_path)
            .with_context(|| format!("Change '{id}' has no expected worktree"))?;
        if target.path == primary {
            bail!("cannot archive the primary worktree");
        }
        if target.prunable || !target.path.exists() {
            bail!("worktree is missing: {}", target.path.display());
        }
        if target.locked {
            bail!("worktree is locked: {}", target.path.display());
        }
        if !force && self.is_dirty(&target.path)? {
            bail!(
                "worktree has uncommitted changes: {}",
                target.path.display()
            );
        }

        let path = target.path.clone();
        let expected_head_oid = target.head_oid.clone();
        let base = Lineage::from_record(&record).base(self)?;
        let target_ref = base.removal_ref.clone();
        let target_oid = target_ref
            .as_deref()
            .map(|reference| self.peel_commit(reference))
            .transpose()?;
        let local_branch = match &target.branch {
            Some(branch) if !self.branch_has_configured_upstream(branch)? => Some(branch.clone()),
            _ => None,
        };
        if !force && !self.tip_integrated(&expected_head_oid, &base)? {
            bail!("Change '{id}' is not merged; use --force to discard it");
        }

        Ok(PreparedArchive {
            path,
            id: id.to_owned(),
            capsule,
            primary,
            expected_head_oid,
            target_oid,
            target_ref,
            local_branch,
            force,
        })
    }

    fn prepare_sync_archive(
        &self,
        id: &str,
        path: &Path,
        head_oid: &str,
        upstream_ref: &str,
        upstream_oid: &str,
    ) -> Result<Option<PreparedArchive>> {
        let worktrees = self.worktrees()?;
        let primary = worktrees
            .first()
            .context("repository has no worktrees")?
            .path
            .clone();
        let (capsule, record) = self
            .repository()?
            .record(id)?
            .with_context(|| format!("Change record is missing for '{id}'"))?;
        if record.id != id || !record.state.is_active() {
            bail!("Change '{id}' is not active");
        }
        let expected_path = capsule.join("workspace");
        if path != expected_path {
            bail!("Change '{id}' worktree path changed during sync");
        }
        let worktree = managed_worktree(&worktrees, &expected_path)
            .with_context(|| format!("Change '{id}' has no expected worktree"))?;
        if worktree.head_oid != head_oid {
            bail!("Change '{id}' HEAD changed during sync");
        }
        if self.same_tree(&record.base_oid, upstream_oid)?
            && !self.same_tree(head_oid, upstream_oid)?
        {
            return Ok(None);
        }
        let integrated = self.is_ancestor(head_oid, upstream_oid)?
            || self.same_tree(head_oid, upstream_oid)?
            || self.merge_adds_no_change(head_oid, upstream_oid, Some(&record.base_oid))?;
        if !integrated {
            return Ok(None);
        }
        Ok(Some(PreparedArchive {
            path: path.to_owned(),
            id: id.to_owned(),
            capsule,
            primary,
            expected_head_oid: head_oid.to_owned(),
            target_oid: Some(upstream_oid.to_owned()),
            target_ref: Some(upstream_ref.to_owned()),
            local_branch: match &worktree.branch {
                Some(branch) if !self.branch_has_configured_upstream(branch)? => {
                    Some(branch.clone())
                }
                _ => None,
            },
            force: false,
        }))
    }

    pub(crate) fn finish_archive(&self, prepared: PreparedArchive) -> Result<()> {
        self.validate_archive_state(&prepared)?;
        mark_closing(
            &prepared.capsule,
            &prepared.id,
            Closing {
                outcome: if prepared.force {
                    Outcome::Discarded
                } else {
                    Outcome::Integrated
                },
                tip_oid: prepared.expected_head_oid.clone(),
                target_oid: prepared.target_oid.clone(),
                target_ref: prepared.target_ref.clone(),
                local_branch: prepared.local_branch.clone(),
            },
        )?;
        self.validate_archive_state(&prepared)?;
        self.worktree_remove(&prepared.path, prepared.force)?;
        if let Some(branch) = &prepared.local_branch {
            self.cleanup_local_branch(&prepared.primary, branch, &prepared.expected_head_oid)?;
        }
        mark_archived(&prepared.capsule, &prepared.id)
            .context("Change worktree was removed, but its archive record did not close")?;
        Ok(())
    }

    fn validate_archive_state(&self, prepared: &PreparedArchive) -> Result<()> {
        let worktrees = self.worktrees()?;
        let worktree = managed_worktree(&worktrees, &prepared.path)
            .with_context(|| format!("Change '{}' expected worktree is gone", prepared.id))?;
        if worktree.prunable || !worktree.path.exists() {
            bail!("Change '{}' expected worktree is gone", prepared.id);
        }
        let live_oid = self.peel_commit_at(&prepared.path, "HEAD")?;
        if worktree.head_oid != prepared.expected_head_oid || live_oid != prepared.expected_head_oid
        {
            bail!("Change '{}' HEAD changed before archive", prepared.id);
        }
        self.validate_target(prepared)
    }

    fn validate_target(&self, prepared: &PreparedArchive) -> Result<()> {
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
                bail!("integration target '{reference}' changed during archive");
            }
        }
        Ok(())
    }

    fn branch_exists(&self, branch: &str) -> Result<bool> {
        self.branch_exists_at(&self.cwd, branch)
    }

    fn branch_exists_at(&self, cwd: &Path, branch: &str) -> Result<bool> {
        let args = [
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ];
        let output = self.raw_at(cwd, &args)?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => check(output, &args).map(|_| false),
        }
    }

    fn branch_has_configured_upstream(&self, branch: &str) -> Result<bool> {
        self.branch_has_configured_upstream_at(&self.cwd, branch)
    }

    fn branch_has_configured_upstream_at(&self, cwd: &Path, branch: &str) -> Result<bool> {
        for key in [
            format!("branch.{branch}.remote"),
            format!("branch.{branch}.merge"),
        ] {
            let args = ["config", "--get", key.as_str()];
            let output = self.raw_at(cwd, &args)?;
            match output.status.code() {
                Some(0) => {}
                Some(1) => return Ok(false),
                _ => {
                    check(output, &args)?;
                    unreachable!()
                }
            }
        }
        Ok(true)
    }

    fn cleanup_local_branch(&self, cwd: &Path, branch: &str, expected: &str) -> Result<()> {
        if !self.branch_exists_at(cwd, branch)?
            || self.branch_has_configured_upstream_at(cwd, branch)?
            || self
                .worktrees_at(cwd)?
                .iter()
                .any(|worktree| worktree.branch.as_deref() == Some(branch))
        {
            return Ok(());
        }
        let reference = format!("refs/heads/{branch}");
        let args = ["update-ref", "-d", reference.as_str(), expected];
        let output = self.raw_at(cwd, &args)?;
        check(output, &args)
            .with_context(|| format!("local branch '{branch}' changed before cleanup"))?;
        Ok(())
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

    fn repository(&self) -> Result<RepositoryDirectory> {
        let common_dir = self.common_dir()?;
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
        RepositoryDirectory::new(repo, common_dir)
    }

    fn worktrees(&self) -> Result<Vec<Worktree>> {
        self.worktrees_at(&self.cwd)
    }

    fn worktrees_at(&self, cwd: &Path) -> Result<Vec<Worktree>> {
        let bytes = self.output_bytes_at(cwd, &["worktree", "list", "--porcelain", "-z"])?;
        bytes
            .split(|byte| *byte == 0)
            .collect::<Vec<_>>()
            .split(|field| field.is_empty())
            .filter(|record| !record.is_empty())
            .map(|record| {
                let mut path = None;
                let mut head_oid = None;
                let mut branch = None;
                let mut locked = false;
                let mut prunable = false;
                for field in record {
                    if let Some(value) = field.strip_prefix(b"worktree ") {
                        path = Some(path_from_bytes(value)?);
                    } else if let Some(value) = field.strip_prefix(b"HEAD ") {
                        head_oid = Some(
                            String::from_utf8(value.to_vec())
                                .context("Git returned a non-UTF-8 worktree HEAD OID")?,
                        );
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
                    head_oid: head_oid.context("Git returned a worktree without a HEAD OID")?,
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

    fn tip_integrated(&self, tip_oid: &str, base: &BranchBase) -> Result<bool> {
        if !base.valid {
            bail!("Change has invalid Grove lineage; use --force to discard it");
        }
        let comparison = base
            .removal_ref
            .as_deref()
            .context("the default branch cannot be archived as a linked worktree")?;
        if self.is_ancestor(tip_oid, comparison)? || self.same_tree(tip_oid, comparison)? {
            return Ok(true);
        }
        self.merge_adds_no_change(tip_oid, comparison, None)
    }

    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        self.predicate(&["merge-base", "--is-ancestor", ancestor, descendant])
    }

    fn worktree_add_detached(&self, path: &Path, oid: &str) -> Result<()> {
        self.output_os(&["worktree", "add", "--detach"], path, &[oid])
    }

    fn worktree_remove(&self, path: &Path, force: bool) -> Result<()> {
        let before = if force {
            &["worktree", "remove", "--force", "--force"][..]
        } else {
            &["worktree", "remove"][..]
        };
        self.output_os(before, path, &[])
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

    fn is_full_commit(&self, oid: &str) -> bool {
        self.peel_commit(oid).is_ok_and(|resolved| resolved == oid)
    }

    fn same_tree(&self, branch: &str, base: &str) -> Result<bool> {
        self.predicate(&["diff", "--quiet", branch, base])
    }

    fn merge_adds_no_change(
        &self,
        branch: &str,
        comparison: &str,
        merge_base: Option<&str>,
    ) -> Result<bool> {
        let mut args = vec!["merge-tree", "--write-tree"];
        if let Some(merge_base) = merge_base {
            args.extend(["--merge-base", merge_base]);
        }
        args.extend([comparison, branch]);
        let output = self.raw(&args)?;
        if !output.status.success() {
            return match output.status.code() {
                Some(1) => Ok(false),
                _ => {
                    check(
                        output,
                        &["merge-tree", "--write-tree", "<comparison>", "<branch>"],
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
        Ok(merged_tree == self.text(&["rev-parse", &format!("{comparison}^{{tree}}")])?)
    }

    fn rollback_created_worktree(&self, path: &Path) -> Result<()> {
        if managed_worktree(&self.worktrees()?, path).is_some() {
            self.worktree_remove(path, true)
                .context("failed to roll back created worktree")?;
        }
        Ok(())
    }

    fn text_at(&self, cwd: &Path, args: &[&str]) -> Result<String> {
        self.checked_at(cwd, args)
            .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_owned())
    }

    fn config_optional_at(&self, cwd: &Path, key: &str) -> Result<Option<String>> {
        let args = ["config", "--get", key];
        let output = self.raw_at(cwd, &args)?;
        match output.status.code() {
            Some(0) => Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            )),
            Some(1) => Ok(None),
            _ => {
                check(output, &args)?;
                unreachable!()
            }
        }
    }

    fn symbolic_head_at(&self, cwd: &Path) -> Result<Option<String>> {
        let args = ["symbolic-ref", "--quiet", "HEAD"];
        let output = self.raw_at(cwd, &args)?;
        match output.status.code() {
            Some(0) => Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            )),
            Some(1) => Ok(None),
            _ => {
                check(output, &args)?;
                unreachable!()
            }
        }
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

fn managed_worktree<'a>(worktrees: &'a [Worktree], expected_path: &Path) -> Option<&'a Worktree> {
    worktrees
        .iter()
        .find(|worktree| worktree.path == expected_path)
}

fn unavailable_worktree_reason(worktrees: &[Worktree], expected_path: &Path) -> &'static str {
    if managed_worktree(worktrees, expected_path).is_some() {
        "worktree is unavailable"
    } else {
        "managed worktree is missing"
    }
}

fn abbreviate_oid(oid: &str) -> String {
    oid.chars().take(12).collect()
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

fn network_remote(url: &str) -> Result<String> {
    if url.contains(['?', '#']) || url.chars().any(char::is_whitespace) {
        bail!("push remote URL contains unsafe information");
    }
    if let Some((scheme, location)) = url.split_once("://") {
        if !matches!(scheme, "http" | "https" | "ssh" | "git") {
            bail!("cannot ship without a network push remote");
        }
        let (authority, path) = location
            .split_once('/')
            .context("cannot ship without a network push remote")?;
        let host = authority.rsplit('@').next().unwrap_or_default();
        if !safe_remote_part(host, false) || !safe_remote_part(path, true) {
            bail!("cannot ship without a network push remote");
        }
        return Ok(format!("{scheme}://{host}/{path}"));
    }
    if let Some((identity, path)) = url.split_once(':')
        && let Some((_, host)) = identity.rsplit_once('@')
        && safe_remote_part(host, false)
        && safe_remote_part(path, true)
    {
        return Ok(format!("{host}:{path}"));
    }
    bail!("cannot ship without a network push remote")
}

fn safe_remote_part(value: &str, path: bool) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '.' | '-' | '_' | '~' | '%' | ':' | '[' | ']')
                || path && character == '/'
        })
}

fn check(output: Output, args: &[&str]) -> Result<Vec<u8>> {
    if output.status.success() {
        return Ok(output.stdout);
    }
    let message = String::from_utf8_lossy(&output.stderr);
    bail!("git {} failed: {}", args.join(" "), message.trim())
}
