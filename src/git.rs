use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::{ffi::OsString, os::unix::ffi::OsStringExt};

use anyhow::{Context, Result, bail};

pub struct Git {
    cwd: PathBuf,
}

pub(crate) struct WorktreeIdentity {
    pub(crate) common_dir: PathBuf,
    pub(crate) root: PathBuf,
}

#[derive(Debug)]
struct Worktree {
    path: PathBuf,
    branch: Option<String>,
    locked: bool,
    prunable: bool,
}

pub struct Status {
    pub changed: bool,
    pub added: usize,
    pub deleted: usize,
    pub conflicts: usize,
}

pub struct Divergence {
    pub ahead: usize,
    pub behind: usize,
}

pub struct Change {
    pub id: String,
    pub path: PathBuf,
}

pub enum WorktreeState {
    Present(Status),
    Missing,
}

pub struct WorktreeView {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_change: bool,
    pub title: Option<String>,
    pub base: String,
    pub divergence: Option<Divergence>,
    pub state: WorktreeState,
    pub current: bool,
    pub primary: bool,
}

pub struct Removal {
    pub label: String,
    pub navigate_to: Option<PathBuf>,
}

pub(crate) struct PreparedRemoval {
    path: PathBuf,
    branch: Option<String>,
    primary: PathBuf,
    current: PathBuf,
    expected_oid: Option<String>,
    force: bool,
    identity: WorktreeIdentity,
}

impl PreparedRemoval {
    pub(crate) fn identity(&self) -> &WorktreeIdentity {
        &self.identity
    }
}

struct BranchBase {
    display: String,
    divergence_ref: Option<String>,
    removal_ref: Option<String>,
    valid: bool,
}

struct CreationBase {
    display_ref: Option<String>,
    oid: String,
    parent: Option<String>,
}

const CHANGE_MARKER_FIELD: &str = "grove-change";
const DESCRIPTION_FIELD: &str = "description";
const BASE_REF_FIELD: &str = "grove-base-ref";
const BASE_OID_FIELD: &str = "grove-base-oid";
const PARENT_FIELD: &str = "grove-parent";
const CHANGE_METADATA_FIELDS: [&str; 5] = [
    CHANGE_MARKER_FIELD,
    DESCRIPTION_FIELD,
    BASE_REF_FIELD,
    BASE_OID_FIELD,
    PARENT_FIELD,
];

struct ChangeMetadata {
    is_change: bool,
    description: Option<String>,
    base_ref: Option<String>,
    base_oid: Option<String>,
    parent: Option<String>,
}

impl ChangeMetadata {
    fn create(git: &Git, source: Option<&str>, description: Option<&str>) -> Result<Change> {
        let base = Self::resolve_creation_base(git, source)?;
        let id = git.change_id()?;
        let path = git.switch(&id, Some(&base))?;
        if let Err(error) = Self::record(git, &id, &base, description) {
            git.rollback_created_worktree(&path, &id);
            return Err(error).context("could not record change metadata");
        }
        Ok(Change { id, path })
    }

    fn resolve_creation_base(git: &Git, source: Option<&str>) -> Result<CreationBase> {
        let Some(source) = source else {
            let default = git.default_branch()?;
            return Ok(CreationBase {
                oid: git.peel_commit(&default)?,
                display_ref: None,
                parent: None,
            });
        };

        if source == "@" {
            let oid = git.peel_commit("HEAD")?;
            let parent = git
                .text(&["symbolic-ref", "--quiet", "--short", "HEAD"])
                .ok();
            return Ok(CreationBase {
                display_ref: Some(source.to_owned()),
                oid,
                parent,
            });
        }

        let oid = git.peel_commit(source)?;
        let parent = git.local_branch(source)?;
        Ok(CreationBase {
            display_ref: Some(source.to_owned()),
            oid,
            parent,
        })
    }

    fn record(
        git: &Git,
        branch: &str,
        base: &CreationBase,
        description: Option<&str>,
    ) -> Result<()> {
        let (base_ref, base_oid, parent) = match &base.display_ref {
            Some(base_ref) => (
                Some(base_ref.as_str()),
                Some(base.oid.as_str()),
                base.parent.as_deref(),
            ),
            None => (None, None, None),
        };
        let fields = [
            (CHANGE_MARKER_FIELD, Some("true")),
            (DESCRIPTION_FIELD, description),
            (BASE_REF_FIELD, base_ref),
            (BASE_OID_FIELD, base_oid),
            (PARENT_FIELD, parent),
        ];
        for (field, value) in fields {
            if let Some(value) = value {
                git.output(&["config", "--local", &metadata_key(branch, field), value])?;
            }
        }
        Ok(())
    }

    fn load(git: &Git, branch: &str) -> Result<Self> {
        let is_change = git
            .config_value(&metadata_key(branch, CHANGE_MARKER_FIELD))?
            .as_deref()
            == Some("true");
        let description = if is_change {
            git.config_value(&metadata_key(branch, DESCRIPTION_FIELD))?
        } else {
            None
        };
        let base_ref = git.config_value(&metadata_key(branch, BASE_REF_FIELD))?;
        let base_oid = git.config_value(&metadata_key(branch, BASE_OID_FIELD))?;
        let parent = git.config_value(&metadata_key(branch, PARENT_FIELD))?;
        Ok(Self {
            is_change,
            description,
            base_ref,
            base_oid,
            parent,
        })
    }

    fn clear(git: &Git, cwd: &Path, branch: &str) -> Result<()> {
        for field in CHANGE_METADATA_FIELDS {
            let key = metadata_key(branch, field);
            let output = git.raw_at(cwd, &["config", "--local", "--unset-all", &key])?;
            if !output.status.success() && output.status.code() != Some(5) {
                check(output, &["config", "--local", "--unset-all", "<key>"])?;
            }
        }
        Ok(())
    }

    fn title(&self) -> Option<String> {
        if !self.is_change {
            return None;
        }
        self.description.as_deref().and_then(|description| {
            description
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_owned)
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
                display: "invalid metadata".to_owned(),
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

fn metadata_key(branch: &str, field: &str) -> String {
    format!("branch.{branch}.{field}")
}

impl Git {
    pub fn discover() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let git = Self { cwd };
        git.text(&["rev-parse", "--git-dir"])
            .context("not inside a Git repository")?;
        if git.text(&["rev-parse", "--is-bare-repository"])? == "true" {
            bail!("bare repositories are not supported");
        }
        Ok(git)
    }

    pub fn project_root(&self) -> Result<PathBuf> {
        self.current_root()
    }

    pub fn worktree_identity(&self) -> Result<WorktreeIdentity> {
        let common_dir = PathBuf::from(self.text(&["rev-parse", "--git-common-dir"])?);
        let common_dir = if common_dir.is_absolute() {
            common_dir
        } else {
            self.cwd.join(common_dir)
        };
        Ok(WorktreeIdentity {
            common_dir: common_dir
                .canonicalize()
                .context("failed to resolve Git common directory")?,
            root: self.current_root()?,
        })
    }

    pub fn enter(&self, branch: &str) -> Result<PathBuf> {
        self.switch(branch, None)
    }

    pub fn create_change(&self, from: Option<&str>, description: Option<&str>) -> Result<Change> {
        ChangeMetadata::create(self, from, description)
    }

    fn switch(&self, branch: &str, base: Option<&CreationBase>) -> Result<PathBuf> {
        self.validate_branch(branch)?;
        let create = base.is_some();
        let worktrees = self.worktrees()?;
        if let Some(worktree) = worktrees
            .iter()
            .find(|worktree| worktree.branch.as_deref() == Some(branch))
        {
            if create {
                bail!("branch '{branch}' already exists");
            }
            return Ok(worktree.path.clone());
        }

        let branch_exists = self.branch_exists(branch)?;
        if create && branch_exists {
            bail!("branch '{branch}' already exists");
        }
        if !create && !branch_exists {
            bail!("branch '{branch}' does not exist; create a change with `grove new`");
        }

        let path = self.worktree_path(branch)?;
        if path.exists() {
            bail!("worktree path already exists: {}", path.display());
        }
        std::fs::create_dir_all(path.parent().context("worktree path has no parent")?)?;

        if let Some(base) = base {
            self.worktree_add_new(&path, branch, base)?;
        } else {
            self.worktree_add(&path, branch)?;
        }
        Ok(path)
    }

    pub fn inventory(&self) -> Result<Vec<WorktreeView>> {
        let worktrees = self.worktrees()?;
        let current = self.current_root()?;
        let primary = worktrees
            .first()
            .context("repository has no worktrees")?
            .path
            .clone();

        worktrees
            .into_iter()
            .map(|worktree| {
                let branch = worktree.branch.clone();
                let metadata = branch
                    .as_deref()
                    .map(|branch| ChangeMetadata::load(self, branch))
                    .transpose()?;
                let is_change = metadata.as_ref().is_some_and(|metadata| metadata.is_change);
                let title = metadata.as_ref().and_then(ChangeMetadata::title);
                let (base, divergence) = if worktree.prunable || branch.is_none() {
                    (String::new(), None)
                } else {
                    let branch = branch.as_deref().context("branch is missing")?;
                    let base = metadata
                        .as_ref()
                        .context("branch metadata is missing")?
                        .base(self, branch)?;
                    let divergence = base
                        .divergence_ref
                        .as_deref()
                        .map(|reference| self.divergence(&worktree.path, reference))
                        .transpose()?;
                    (base.display, divergence)
                };
                let state = if worktree.prunable {
                    WorktreeState::Missing
                } else {
                    WorktreeState::Present(self.status(&worktree.path)?)
                };
                Ok(WorktreeView {
                    current: worktree.path == current,
                    primary: worktree.path == primary,
                    path: worktree.path,
                    branch,
                    is_change,
                    title,
                    base,
                    divergence,
                    state,
                })
            })
            .collect()
    }

    pub(crate) fn prepare_removal(
        &self,
        requested: Option<&str>,
        force: bool,
    ) -> Result<PreparedRemoval> {
        let worktrees = self.worktrees()?;
        let primary = worktrees
            .first()
            .context("repository has no worktrees")?
            .path
            .clone();
        let current = self.current_root()?;
        let target = match requested {
            Some(branch) => worktrees
                .iter()
                .find(|worktree| worktree.branch.as_deref() == Some(branch))
                .with_context(|| format!("branch '{branch}' has no worktree"))?,
            None => worktrees
                .iter()
                .find(|worktree| worktree.path == current)
                .context("current directory is not in a worktree")?,
        };
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
        let branch = target.branch.clone();
        let expected_oid = if force {
            None
        } else {
            branch
                .as_deref()
                .map(|branch| self.branch_oid(branch))
                .transpose()?
        };
        if !force && let Some(branch) = &branch {
            let base = ChangeMetadata::load(self, branch)?.base(self, branch)?;
            if !self.branch_integrated(branch, &base)? {
                bail!("branch '{branch}' is not merged; use --force to discard it");
            }
        }

        let common_dir = self.worktree_identity()?.common_dir;
        let root = if path
            .try_exists()
            .with_context(|| format!("failed to inspect worktree {}", path.display()))?
        {
            path.canonicalize()
                .with_context(|| format!("failed to resolve worktree {}", path.display()))?
        } else {
            path.clone()
        };
        Ok(PreparedRemoval {
            path,
            branch,
            primary,
            current,
            expected_oid,
            force,
            identity: WorktreeIdentity { common_dir, root },
        })
    }

    pub(crate) fn remove(&self, prepared: PreparedRemoval) -> Result<Removal> {
        self.worktree_remove(&prepared.path, prepared.force)?;
        if let Some(branch) = &prepared.branch {
            self.delete_branch(&prepared.primary, branch, prepared.expected_oid.as_deref())
                .context("worktree was removed, but branch cleanup did not complete")?;
        }
        Ok(Removal {
            label: prepared.branch.unwrap_or_else(|| "detached".to_owned()),
            navigate_to: (prepared.path == prepared.current).then_some(prepared.primary),
        })
    }

    pub fn branch_names(&self, worktrees_only: bool) -> Result<Vec<String>> {
        if worktrees_only {
            return Ok(self
                .worktrees()?
                .into_iter()
                .filter_map(|worktree| worktree.branch)
                .collect());
        }
        self.branches()
    }

    fn validate_branch(&self, branch: &str) -> Result<()> {
        self.output(&["check-ref-format", "--branch", branch])?;
        Ok(())
    }

    fn change_id(&self) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before the Unix epoch")?
            .as_nanos();
        for nonce in 0..100_u8 {
            let seed = format!(
                "{}:{now}:{}:{nonce}",
                self.cwd.display(),
                std::process::id()
            );
            let digest = blake3::hash(seed.as_bytes()).to_hex();
            let id = format!("c-{}", &digest[..12]);
            if !self.branch_exists(&id)? {
                return Ok(id);
            }
        }
        bail!("could not generate a unique change ID")
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

    fn worktree_path(&self, branch: &str) -> Result<PathBuf> {
        let common_dir = PathBuf::from(self.text(&["rev-parse", "--git-common-dir"])?);
        let common_dir = if common_dir.is_absolute() {
            common_dir
        } else {
            self.cwd.join(common_dir)
        };
        let common_dir = common_dir
            .canonicalize()
            .context("failed to resolve Git common directory")?;
        let primary = self
            .worktrees()?
            .into_iter()
            .next()
            .context("repository has no worktrees")?;
        let repo = primary
            .path
            .file_name()
            .context("primary worktree has no directory name")?
            .to_string_lossy();
        let digest = blake3::hash(common_dir.as_os_str().as_encoded_bytes()).to_hex();
        let home = std::env::var_os("HOME").context("HOME is not set")?;

        Ok(PathBuf::from(home)
            .join(".grove")
            .join(format!("{}-{}", encode_path_segment(&repo), &digest[..12]))
            .join(encode_path_segment(branch)))
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
            bail!("branch '{branch}' has invalid Grove base metadata; use --force to discard it");
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

    fn branches(&self) -> Result<Vec<String>> {
        Ok(self
            .text(&["for-each-ref", "--format=%(refname:short)", "refs/heads"])?
            .lines()
            .map(str::to_owned)
            .collect())
    }

    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        self.predicate(&["merge-base", "--is-ancestor", ancestor, descendant])
    }

    fn branch_oid(&self, branch: &str) -> Result<String> {
        self.text(&["rev-parse", &format!("refs/heads/{branch}")])
    }

    fn worktree_add_new(&self, path: &Path, branch: &str, base: &CreationBase) -> Result<()> {
        self.output_os(&["worktree", "add", "-b", branch], path, &[&base.oid])
    }

    fn worktree_add(&self, path: &Path, branch: &str) -> Result<()> {
        self.output_os(&["worktree", "add"], path, &[branch])
    }

    fn worktree_remove(&self, path: &Path, force: bool) -> Result<()> {
        let before = if force {
            &["worktree", "remove", "--force", "--force"][..]
        } else {
            &["worktree", "remove"][..]
        };
        self.output_os(before, path, &[])
    }

    fn delete_branch(&self, cwd: &Path, branch: &str, expected: Option<&str>) -> Result<()> {
        let reference = format!("refs/heads/{branch}");
        let mut command = Command::new("git");
        command.arg("-C").arg(cwd);
        let shown;
        if let Some(expected) = expected {
            command.args(["update-ref", "-d", &reference, expected]);
            shown = vec!["update-ref", "-d", "<branch>", "<expected>"];
        } else {
            command.args(["branch", "-D", "--", branch]);
            shown = vec!["branch", "-D", "--", "<branch>"];
        }
        let output = command.output().context("could not delete branch")?;
        check(output, &shown).with_context(|| {
            if expected.is_some() {
                format!("branch '{branch}' changed before it could be deleted")
            } else {
                format!("branch '{branch}' could not be deleted")
            }
        })?;
        ChangeMetadata::clear(self, cwd, branch)
            .context("branch was deleted, but its Grove change metadata could not be cleared")
    }

    fn text(&self, args: &[&str]) -> Result<String> {
        self.text_at(&self.cwd, args)
    }

    fn peel_commit(&self, source: &str) -> Result<String> {
        let revision = format!("{source}^{{commit}}");
        let args = [
            "rev-parse",
            "--verify",
            "--end-of-options",
            revision.as_str(),
        ];
        let output = self.raw(&args)?;
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

    fn config_value(&self, key: &str) -> Result<Option<String>> {
        let output = self.raw(&["config", "--local", "--get", key])?;
        match output.status.code() {
            Some(0) => Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            )),
            Some(1) => Ok(None),
            _ => {
                check(output, &["config", "--local", "--get", "<key>"])?;
                unreachable!()
            }
        }
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

    fn rollback_created_worktree(&self, path: &Path, branch: &str) {
        let _ = ChangeMetadata::clear(self, &self.cwd, branch);
        let _ = self.worktree_remove(path, true);
        let _ = self.raw(&["update-ref", "-d", &format!("refs/heads/{branch}")]);
    }

    fn text_at(&self, cwd: &Path, args: &[&str]) -> Result<String> {
        self.checked_at(cwd, args)
            .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_owned())
    }

    fn output(&self, args: &[&str]) -> Result<()> {
        self.checked_at(&self.cwd, args).map(|_| ())
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

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
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

fn check(output: Output, args: &[&str]) -> Result<Vec<u8>> {
    if output.status.success() {
        return Ok(output.stdout);
    }
    let message = String::from_utf8_lossy(&output.stderr);
    bail!("git {} failed: {}", args.join(" "), message.trim())
}
