use std::{path::Path, process::Command};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Repository {
    pub(crate) owner: String,
    pub(crate) name: String,
}

impl Repository {
    pub(crate) fn parse(remote: &str) -> Result<Self> {
        let path = if let Some(path) = remote.strip_prefix("https://github.com/") {
            path
        } else if let Some(path) = remote.strip_prefix("git@github.com:") {
            path
        } else if let Some(path) = remote.strip_prefix("ssh://git@github.com/") {
            path
        } else {
            bail!("unsupported GitHub remote URL");
        };

        let path = path.strip_suffix(".git").unwrap_or(path);
        let mut parts = path.split('/');
        let owner = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        if owner.is_empty()
            || name.is_empty()
            || parts.next().is_some()
            || !owner.chars().all(repository_character)
            || !name.chars().all(repository_character)
        {
            bail!("invalid GitHub repository URL");
        }
        Ok(Self {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }

    pub(crate) fn name_with_owner(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

fn repository_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
}

pub(crate) struct Github {
    pub(crate) repository: Repository,
}

pub(crate) struct PullRequest {
    pub(crate) url: String,
    pub(crate) summary: String,
}

impl Github {
    pub(crate) fn preflight(push_url: &str, cwd: &Path) -> Result<Self> {
        let repository = Repository::parse(push_url)?;
        run_gh(
            cwd,
            &["auth", "status", "--active", "--hostname", "github.com"],
            "GitHub authentication check failed",
        )?;
        let name = repository.name_with_owner();
        let output = run_gh(
            cwd,
            &["repo", "view", &name, "--json", "nameWithOwner"],
            &format!("cannot access GitHub repository {name}"),
        )?;
        let viewed: RepositoryView = serde_json::from_slice(&output)
            .with_context(|| format!("invalid response while viewing GitHub repository {name}"))?;
        let viewed = Repository::from_name_with_owner(&viewed.name_with_owner)
            .context("GitHub returned an invalid repository name")?;
        if !viewed
            .name_with_owner()
            .eq_ignore_ascii_case(&repository.name_with_owner())
        {
            bail!(
                "GitHub returned repository {}, expected {name}",
                viewed.name_with_owner()
            );
        }
        Ok(Self { repository: viewed })
    }

    pub(crate) fn pull_request(
        &self,
        cwd: &Path,
        branch: &str,
        expected_head: &str,
    ) -> Result<PullRequest> {
        let name = self.repository.name_with_owner();
        let output = run_gh(
            cwd,
            &[
                "pr",
                "view",
                branch,
                "--repo",
                &name,
                "--json",
                "url,state,headRefName,headRefOid,isCrossRepository,reviewDecision,statusCheckRollup",
            ],
            &format!("cannot view pull request for branch '{branch}' in {name}"),
        )?;
        let pull_request: PullRequestView = serde_json::from_slice(&output).with_context(|| {
            format!("invalid pull request response for branch '{branch}' in {name}")
        })?;
        if pull_request.state != "OPEN" {
            bail!("pull request {} is not open", pull_request.url);
        }
        if pull_request.is_cross_repository {
            bail!("pull request {} is from a fork", pull_request.url);
        }
        if pull_request.head_ref_name != branch {
            bail!(
                "pull request {} has head branch '{}', expected '{branch}'",
                pull_request.url,
                pull_request.head_ref_name
            );
        }
        if pull_request.head_ref_oid != expected_head {
            bail!(
                "pull request {} has head {}, expected {expected_head}",
                pull_request.url,
                pull_request.head_ref_oid
            );
        }
        let prefix = format!("https://github.com/{name}/pull/");
        if !pull_request
            .url
            .strip_prefix(&prefix)
            .is_some_and(|number| {
                !number.is_empty() && number.chars().all(|digit| digit.is_ascii_digit())
            })
        {
            bail!("GitHub returned an invalid pull request URL");
        }
        Ok(PullRequest {
            url: pull_request.url,
            summary: summarize(
                pull_request.review_decision.as_deref(),
                &pull_request.status_check_rollup,
            ),
        })
    }
}

impl Repository {
    fn from_name_with_owner(name: &str) -> Result<Self> {
        Self::parse(&format!("https://github.com/{name}"))
    }
}

fn run_gh(cwd: &Path, arguments: &[&str], message: &str) -> Result<Vec<u8>> {
    let output = Command::new("gh")
        .args(arguments)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to run gh: {message}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        let detail = detail.trim();
        if detail.is_empty() {
            bail!("{message}: gh exited with {}", output.status);
        }
        bail!("{message}: {detail}");
    }
    Ok(output.stdout)
}

#[derive(Deserialize)]
struct RepositoryView {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
}

#[derive(Deserialize)]
struct PullRequestView {
    url: String,
    state: String,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
    #[serde(rename = "isCrossRepository")]
    is_cross_repository: bool,
    #[serde(rename = "reviewDecision")]
    review_decision: Option<String>,
    #[serde(rename = "statusCheckRollup", default)]
    status_check_rollup: Vec<Value>,
}

fn summarize(review: Option<&str>, checks: &[Value]) -> String {
    let review = match review {
        Some("APPROVED") => "approved",
        Some("CHANGES_REQUESTED") => "changes requested",
        Some("REVIEW_REQUIRED") => "review required",
        Some("") | None => "no review decision",
        Some(_) => "review pending",
    };
    let mut pending = false;
    let mut failed = false;
    for check in checks {
        let state = check
            .get("conclusion")
            .and_then(Value::as_str)
            .filter(|state| !state.is_empty())
            .or_else(|| check.get("state").and_then(Value::as_str))
            .filter(|state| !state.is_empty());
        let status = check.get("status").and_then(Value::as_str);
        if state.is_some_and(|state| matches!(state, "PENDING" | "EXPECTED")) {
            pending = true;
        } else if state.is_some_and(|state| !matches!(state, "SUCCESS" | "NEUTRAL" | "SKIPPED")) {
            failed = true;
        } else if state.is_none() || status.is_some_and(|status| status != "COMPLETED") {
            pending = true;
        }
    }
    let checks = if checks.is_empty() {
        "no checks"
    } else if failed {
        "checks failing"
    } else if pending {
        "checks pending"
    } else {
        "checks passing"
    };
    format!("{review}; {checks}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_github_remote_urls() {
        for remote in [
            "https://github.com/owner/repo.git",
            "git@github.com:owner/repo.git",
            "ssh://git@github.com/owner/repo.git",
        ] {
            assert_eq!(
                Repository::parse(remote).unwrap(),
                Repository {
                    owner: "owner".to_owned(),
                    name: "repo".to_owned(),
                }
            );
        }
    }

    #[test]
    fn rejects_nonstandard_or_non_github_remotes() {
        for remote in [
            "http://github.com/owner/repo.git",
            "https://example.com/owner/repo.git",
            "ssh://other@github.com/owner/repo.git",
            "https://github.com/owner/repo/extra",
            "https://github.com/owner/repo?tab=readme",
        ] {
            assert!(Repository::parse(remote).is_err(), "accepted {remote}");
        }
    }

    #[test]
    fn summarizes_review_and_check_rollup() {
        assert_eq!(summarize(Some("APPROVED"), &[]), "approved; no checks");
        assert_eq!(
            summarize(
                Some("REVIEW_REQUIRED"),
                &[json!({"status": "COMPLETED", "conclusion": "SUCCESS"})]
            ),
            "review required; checks passing"
        );
        assert_eq!(
            summarize(
                Some(""),
                &[
                    json!({"state": "EXPECTED"}),
                    json!({"status": "IN_PROGRESS", "conclusion": ""}),
                ]
            ),
            "no review decision; checks pending"
        );
        assert_eq!(
            summarize(
                Some("CHANGES_REQUESTED"),
                &[json!({"state": "FAILURE"}), json!({"state": "PENDING"})]
            ),
            "changes requested; checks failing"
        );
    }
}
