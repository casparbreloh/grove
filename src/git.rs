use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[cfg(unix)]
use std::{ffi::OsString, os::unix::ffi::OsStringExt};

use anyhow::{Context, Result, bail};

pub struct Git {
    cwd: PathBuf,
}

#[derive(Debug)]
pub struct Worktree {
    pub path: PathBuf,
    branch: Option<String>,
    pub locked: bool,
    pub prunable: bool,
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

impl Worktree {
    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }
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

    pub fn validate_branch(&self, branch: &str) -> Result<()> {
        self.output(&["check-ref-format", "--branch", branch])?;
        Ok(())
    }

    pub fn branch_exists(&self, branch: &str) -> Result<bool> {
        let output = self.raw(&[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])?;
        if output.status.success() {
            Ok(true)
        } else if output.status.code() == Some(1) {
            Ok(false)
        } else {
            check(output, &["show-ref", "--verify", "--quiet", "<branch>"])?;
            unreachable!()
        }
    }

    pub fn default_branch(&self) -> Result<String> {
        if let Ok(remote) = self.text(&[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ]) {
            return Ok(remote);
        }
        for branch in ["main", "master"] {
            if self.branch_exists(branch)? {
                return Ok(branch.to_owned());
            }
        }
        if let Some(branch) = self.worktrees()?.first().and_then(Worktree::branch) {
            return Ok(branch.to_owned());
        }
        bail!("could not detect the default branch")
    }

    pub fn current_root(&self) -> Result<PathBuf> {
        Ok(PathBuf::from(self.text(&["rev-parse", "--show-toplevel"])?))
    }

    pub fn worktrees(&self) -> Result<Vec<Worktree>> {
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

    pub fn is_dirty(&self, path: &Path) -> Result<bool> {
        Ok(!self
            .text_at(path, &["status", "--porcelain", "--untracked-files=normal"])?
            .is_empty())
    }

    pub fn status(&self, path: &Path) -> Result<Status> {
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

    pub fn divergence(&self, path: &Path, base: &str) -> Result<Divergence> {
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

    pub fn branches(&self) -> Result<Vec<String>> {
        Ok(self
            .text(&["for-each-ref", "--format=%(refname:short)", "refs/heads"])?
            .lines()
            .map(str::to_owned)
            .collect())
    }

    pub fn branch_merged(&self, branch: &str, base: &str) -> Result<bool> {
        let output = self.raw(&["merge-base", "--is-ancestor", branch, base])?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => {
                check(
                    output,
                    &["merge-base", "--is-ancestor", "<branch>", "<base>"],
                )?;
                unreachable!()
            }
        }
    }

    pub fn branch_oid(&self, branch: &str) -> Result<String> {
        self.text(&["rev-parse", &format!("refs/heads/{branch}")])
    }

    pub fn worktree_add_new(&self, path: &Path, branch: &str, base: &str) -> Result<()> {
        self.output_os(&["worktree", "add", "-b", branch], path, &[base])
    }

    pub fn worktree_add(&self, path: &Path, branch: &str) -> Result<()> {
        self.output_os(&["worktree", "add"], path, &[branch])
    }

    pub fn worktree_remove(&self, path: &Path, force: bool) -> Result<()> {
        let before = if force {
            &["worktree", "remove", "--force", "--force"][..]
        } else {
            &["worktree", "remove"][..]
        };
        self.output_os(before, path, &[])
    }

    pub fn delete_branch(&self, cwd: &Path, branch: &str, expected: Option<&str>) -> Result<()> {
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
        check(output, &shown).map(|_| ())
    }

    fn text(&self, args: &[&str]) -> Result<String> {
        self.text_at(&self.cwd, args)
    }

    fn text_at(&self, cwd: &Path, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .output()
            .with_context(|| format!("could not run git {}", args.join(" ")))?;
        check(output, args).map(|bytes| String::from_utf8_lossy(&bytes).trim().to_owned())
    }

    fn output(&self, args: &[&str]) -> Result<()> {
        check(self.raw(args)?, args).map(|_| ())
    }

    fn output_bytes(&self, args: &[&str]) -> Result<Vec<u8>> {
        self.output_bytes_at(&self.cwd, args)
    }

    fn output_bytes_at(&self, cwd: &Path, args: &[&str]) -> Result<Vec<u8>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .output()
            .with_context(|| format!("could not run git {}", args.join(" ")))?;
        check(output, args)
    }

    fn raw(&self, args: &[&str]) -> Result<Output> {
        Command::new("git")
            .arg("-C")
            .arg(&self.cwd)
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
