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
    pub flags: String,
    pub added: usize,
    pub deleted: usize,
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
        bail!("could not detect a default branch (expected origin/HEAD, main, or master)")
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
        let mut staged = false;
        let mut modified = false;
        let mut untracked = false;
        let mut conflicts = false;
        for line in porcelain.lines() {
            let code = line.as_bytes().get(..2).unwrap_or_default();
            untracked |= code == b"??";
            conflicts |= matches!(code, b"DD" | b"AU" | b"UD" | b"UA" | b"DU" | b"AA" | b"UU");
            staged |= code
                .first()
                .is_some_and(|byte| *byte != b' ' && *byte != b'?');
            modified |= code
                .get(1)
                .is_some_and(|byte| *byte != b' ' && *byte != b'?');
        }

        let mut flags = String::new();
        if conflicts {
            flags.push('✘');
        } else {
            if staged {
                flags.push('+');
            }
            if modified {
                flags.push('!');
            }
            if untracked {
                flags.push('?');
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
        Ok(Status {
            flags,
            added,
            deleted,
        })
    }

    pub fn divergence(&self, path: &Path, base: &str) -> Result<(usize, usize)> {
        let counts = self.text_at(
            path,
            &[
                "rev-list",
                "--left-right",
                "--count",
                &format!("{base}...HEAD"),
            ],
        )?;
        let mut counts = counts.split_whitespace();
        let behind = counts
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let ahead = counts
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        Ok((ahead, behind))
    }

    pub fn commit(&self, path: &Path) -> Result<(String, String)> {
        let text = self.text_at(path, &["log", "-1", "--format=%h%x00%s"])?;
        let (hash, message) = text.split_once('\0').unwrap_or((&text, ""));
        Ok((hash.to_owned(), message.to_owned()))
    }

    pub fn worktree_add_new(&self, path: &Path, branch: &str, base: &str) -> Result<()> {
        self.output_os(&["worktree", "add", "-b", branch], path, &[base])
    }

    pub fn worktree_add(&self, path: &Path, branch: &str) -> Result<()> {
        self.output_os(&["worktree", "add"], path, &[branch])
    }

    pub fn worktree_remove(&self, path: &Path) -> Result<()> {
        self.output_os(&["worktree", "remove"], path, &[])
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
        check(self.raw(args)?, args)
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
