use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::json;

use crate::{change::publication_branch_base, git::Git, session};

pub(crate) fn run(git: &Git) -> Result<()> {
    let change = git
        .current_change()?
        .context("current workspace is not a managed Grove Change")?;
    let title = change
        .title
        .as_deref()
        .context("cannot ship an Untitled Change")?;
    let branch_base = publication_branch_base(title)?;
    let fallback_branch = format!("{branch_base}-{}", change.id);
    let workspace = &change.workspace;
    let push_remote = git.push_remote(workspace)?;
    let code_host = CodeHost::from_remote(&push_remote.url)?;
    let default_target_branch = code_host.preflight()?;
    let (branch, remote_branch_oid) = git.select_ship_branch(
        workspace,
        &push_remote.name,
        &branch_base,
        &fallback_branch,
        change.publication_branch.as_deref(),
        change.published_oid.as_deref(),
    )?;
    let existing_pull_request = code_host.find_pull_request(&branch)?;
    let target_branch = existing_pull_request.as_ref().map_or_else(
        || default_target_branch,
        |pull_request| pull_request.target_branch.clone(),
    );
    let target_ref = git.fetch_ship_base(workspace, &push_remote.name, &target_branch)?;
    let branch_published = existing_pull_request.is_some()
        || remote_branch_oid.is_some()
        || change.published_oid.is_some();
    if !branch_published && !git.has_publishable_work(workspace, &target_ref)? {
        bail!("Change has no work to ship");
    }

    let may_fast_forward_branch = change.publication_branch.as_deref() == Some(branch.as_str());
    change.capsule.initialize_publication_branch(&branch)?;
    git.prepare_ship(workspace, &branch, may_fast_forward_branch)?;
    let snapshot = git.capture_ship_snapshot(workspace, &target_ref)?;
    let pull_request_context = existing_pull_request.as_ref().map_or_else(
        || "There is no open pull request.".to_owned(),
        |pull_request| {
            format!(
                "Current pull request title: {}\nCurrent pull request body: {}",
                pull_request.title, pull_request.body
            )
        },
    );
    let prompt = format!(
        "Change title: {title}\nPublication branch: {branch}\nTarget branch: {target_branch}\nPublished history: {branch_published}\nNew staged work: {}\n{pull_request_context}\n\n{}",
        snapshot.staged, snapshot.summary
    );
    let metadata = session::generate_ship_metadata(workspace, &prompt)?;
    validate_ship_metadata(
        &metadata,
        branch_published,
        snapshot.staged,
        existing_pull_request.is_some(),
    )?;
    if let Some(existing) = &existing_pull_request {
        code_host.validate_pull_request_unchanged(&branch, existing)?;
    }

    git.validate_ship_snapshot(workspace, &branch, &snapshot)?;
    if snapshot.staged {
        let subject = if branch_published {
            metadata.commit.as_deref().context(
                "shipping metadata did not include a commit for newly staged published work",
            )?
        } else {
            &metadata
                .pull_request
                .as_ref()
                .context("shipping metadata did not include pull request metadata")?
                .title
        };
        git.commit_ship(workspace, subject, &snapshot)?;
    }
    let head_oid = git.validate_clean_ship(workspace, &branch)?;
    git.push_ship(
        workspace,
        &push_remote.name,
        &branch,
        &head_oid,
        remote_branch_oid.as_deref(),
    )?;
    change.capsule.mark_published(&branch, &head_oid)?;

    let pull_request = match existing_pull_request {
        Some(existing) => match metadata.pull_request {
            Some(replacement)
                if existing.title != replacement.title || existing.body != replacement.body =>
            {
                code_host.update_pull_request(
                    &branch,
                    &existing,
                    &replacement.title,
                    &replacement.body,
                )?
            }
            _ => existing,
        },
        None => {
            let metadata = metadata
                .pull_request
                .context("shipping metadata did not include pull request metadata")?;
            code_host.create_pull_request(
                &branch,
                &target_branch,
                &metadata.title,
                &metadata.body,
            )?
        }
    };
    println!("{}", pull_request.url);
    Ok(())
}

fn validate_ship_metadata(
    metadata: &session::ShipMetadata,
    branch_published: bool,
    staged: bool,
    has_pull_request: bool,
) -> Result<()> {
    if !branch_published && metadata.commit.is_some() {
        bail!("shipping metadata included an unnecessary initial commit subject");
    }
    if branch_published && staged && metadata.commit.is_none() {
        bail!("shipping metadata did not include a commit for newly staged published work");
    }
    if !staged && metadata.commit.is_some() {
        bail!("shipping metadata included a commit without newly staged work");
    }
    if !has_pull_request && metadata.pull_request.is_none() {
        bail!("shipping metadata did not include pull request metadata");
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PullRequest {
    url: String,
    title: String,
    body: String,
    target_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CodeHost {
    GitHub { repository: String, owner: String },
    GitLab { project: String },
}

#[derive(Deserialize)]
struct GitHubPullRequest {
    number: u64,
    html_url: String,
    title: String,
    body: Option<String>,
    base: GitHubPullRequestBase,
}

#[derive(Deserialize)]
struct GitHubPullRequestBase {
    #[serde(rename = "ref")]
    branch: String,
}

#[derive(Deserialize)]
struct GitLabMergeRequest {
    iid: u64,
    web_url: String,
    title: String,
    description: Option<String>,
    source_project_id: u64,
    target_branch: String,
}

#[derive(Deserialize)]
struct GitHubRepository {
    default_branch: String,
}

#[derive(Deserialize)]
struct GitLabProject {
    id: u64,
    default_branch: String,
}

enum HostPullRequest {
    GitHub(GitHubPullRequest),
    GitLab(GitLabMergeRequest),
}

impl HostPullRequest {
    fn pull_request(&self) -> PullRequest {
        match self {
            Self::GitHub(pull_request) => PullRequest {
                url: pull_request.html_url.clone(),
                title: pull_request.title.clone(),
                body: pull_request.body.clone().unwrap_or_default(),
                target_branch: pull_request.base.branch.clone(),
            },
            Self::GitLab(merge_request) => PullRequest {
                url: merge_request.web_url.clone(),
                title: merge_request.title.clone(),
                body: merge_request.description.clone().unwrap_or_default(),
                target_branch: merge_request.target_branch.clone(),
            },
        }
    }
}

impl CodeHost {
    fn from_remote(remote: &str) -> Result<Self> {
        let (host, path) = remote_parts(remote)?;
        match host.as_str() {
            "github.com" => {
                let mut components = path.split('/');
                let owner = components.next().unwrap_or_default();
                let repository = components.next().unwrap_or_default();
                if owner.is_empty() || repository.is_empty() || components.next().is_some() {
                    bail!("invalid GitHub repository path in remote URL");
                }
                Ok(Self::GitHub {
                    repository: format!("{owner}/{repository}"),
                    owner: owner.to_owned(),
                })
            }
            "gitlab.com" => {
                if path.split('/').count() < 2 {
                    bail!("invalid GitLab project path in remote URL");
                }
                Ok(Self::GitLab { project: path })
            }
            _ => bail!("remote host '{host}' is not supported"),
        }
    }

    fn preflight(&self) -> Result<String> {
        match self {
            Self::GitHub { repository, .. } => {
                run_command(
                    command("gh", &["auth", "status", "--hostname", "github.com"]),
                    "check GitHub authentication",
                )?;
                let output = run_command(
                    command("gh", &["api", &format!("repos/{repository}")]),
                    "check GitHub repository access",
                )?;
                let repository: GitHubRepository = serde_json::from_slice(&output)
                    .context("invalid JSON from gh while checking repository access")?;
                Ok(repository.default_branch)
            }
            Self::GitLab { project } => {
                run_command(
                    command("glab", &["auth", "status", "--hostname", "gitlab.com"]),
                    "check GitLab authentication",
                )?;
                let output = run_command(
                    command(
                        "glab",
                        &[
                            "api",
                            "--hostname",
                            "gitlab.com",
                            &format!("projects/{}", encode_project(project)),
                        ],
                    ),
                    "check GitLab project access",
                )?;
                let project: GitLabProject = serde_json::from_slice(&output)
                    .context("invalid JSON from glab while checking project access")?;
                Ok(project.default_branch)
            }
        }
    }

    fn find_pull_request(&self, source_branch: &str) -> Result<Option<PullRequest>> {
        self.find_host_pull_request(source_branch)?
            .map(|pull_request| self.validate_pull_request(pull_request.pull_request()))
            .transpose()
    }

    fn create_pull_request(
        &self,
        source_branch: &str,
        target_branch: &str,
        title: &str,
        body: &str,
    ) -> Result<PullRequest> {
        match self {
            Self::GitHub { repository, .. } => {
                let endpoint = format!("repos/{repository}/pulls");
                let payload = json!({
                    "title": title,
                    "body": body,
                    "head": source_branch,
                    "base": target_branch,
                });
                let output = run_json_command(
                    command("gh", &["api", "--method", "POST", &endpoint]),
                    &payload,
                    "create GitHub pull request",
                )?;
                let pull_request: GitHubPullRequest = serde_json::from_slice(&output)
                    .context("invalid JSON from gh while creating pull request")?;
                self.validate_pull_request(HostPullRequest::GitHub(pull_request).pull_request())
            }
            Self::GitLab { project } => {
                let endpoint = format!("projects/{}/merge_requests", encode_project(project));
                let payload = json!({
                    "source_branch": source_branch,
                    "target_branch": target_branch,
                    "title": title,
                    "description": body,
                });
                let output = run_json_command(
                    command(
                        "glab",
                        &[
                            "api",
                            "--hostname",
                            "gitlab.com",
                            "--method",
                            "POST",
                            &endpoint,
                        ],
                    ),
                    &payload,
                    "create GitLab merge request",
                )?;
                let merge_request: GitLabMergeRequest = serde_json::from_slice(&output)
                    .context("invalid JSON from glab while creating merge request")?;
                self.validate_pull_request(HostPullRequest::GitLab(merge_request).pull_request())
            }
        }
    }

    fn validate_pull_request_unchanged(
        &self,
        source_branch: &str,
        expected: &PullRequest,
    ) -> Result<()> {
        if self.find_pull_request(source_branch)?.as_ref() != Some(expected) {
            bail!("pull request changed during shipping");
        }
        Ok(())
    }

    fn update_pull_request(
        &self,
        source_branch: &str,
        expected: &PullRequest,
        title: &str,
        body: &str,
    ) -> Result<PullRequest> {
        let host_pull_request = self
            .find_host_pull_request(source_branch)?
            .with_context(|| {
                format!("no open pull request found for source branch '{source_branch}'")
            })?;
        if &host_pull_request.pull_request() != expected {
            bail!("pull request changed during shipping");
        }
        match (self, host_pull_request) {
            (Self::GitHub { repository, .. }, HostPullRequest::GitHub(pull_request)) => {
                let endpoint = format!("repos/{repository}/pulls/{}", pull_request.number);
                let payload = json!({"title": title, "body": body});
                let output = run_json_command(
                    command("gh", &["api", "--method", "PATCH", &endpoint]),
                    &payload,
                    "update GitHub pull request",
                )?;
                let pull_request: GitHubPullRequest = serde_json::from_slice(&output)
                    .context("invalid JSON from gh while updating pull request")?;
                self.validate_pull_request(HostPullRequest::GitHub(pull_request).pull_request())
            }
            (Self::GitLab { project }, HostPullRequest::GitLab(merge_request)) => {
                let endpoint = format!(
                    "projects/{}/merge_requests/{}",
                    encode_project(project),
                    merge_request.iid
                );
                let payload = json!({"title": title, "description": body});
                let output = run_json_command(
                    command(
                        "glab",
                        &[
                            "api",
                            "--hostname",
                            "gitlab.com",
                            "--method",
                            "PUT",
                            &endpoint,
                        ],
                    ),
                    &payload,
                    "update GitLab merge request",
                )?;
                let merge_request: GitLabMergeRequest = serde_json::from_slice(&output)
                    .context("invalid JSON from glab while updating merge request")?;
                self.validate_pull_request(HostPullRequest::GitLab(merge_request).pull_request())
            }
            _ => unreachable!("pull request provider must match code host"),
        }
    }

    fn validate_pull_request(&self, pull_request: PullRequest) -> Result<PullRequest> {
        let expected = match self {
            Self::GitHub { repository, .. } => {
                format!("https://github.com/{repository}/pull/")
            }
            Self::GitLab { project } => {
                format!("https://gitlab.com/{project}/-/merge_requests/")
            }
        };
        if !pull_request.url.starts_with(&expected)
            || pull_request.url[expected.len()..].parse::<u64>().is_err()
        {
            bail!("code host returned an unexpected pull request URL");
        }
        Ok(pull_request)
    }

    fn find_host_pull_request(&self, source_branch: &str) -> Result<Option<HostPullRequest>> {
        let pull_requests: Vec<HostPullRequest> = match self {
            Self::GitHub { repository, owner } => {
                let endpoint = format!("repos/{repository}/pulls");
                let output = run_command(
                    command(
                        "gh",
                        &[
                            "api",
                            "--method",
                            "GET",
                            &endpoint,
                            "-f",
                            "state=open",
                            "-f",
                            &format!("head={owner}:{source_branch}"),
                            "-f",
                            "per_page=2",
                        ],
                    ),
                    "find open GitHub pull request",
                )?;
                let pull_requests: Vec<GitHubPullRequest> = serde_json::from_slice(&output)
                    .context("invalid JSON from gh while finding pull request")?;
                pull_requests
                    .into_iter()
                    .map(HostPullRequest::GitHub)
                    .collect()
            }
            Self::GitLab { project } => {
                let project_endpoint = format!("projects/{}", encode_project(project));
                let output = run_command(
                    command(
                        "glab",
                        &["api", "--hostname", "gitlab.com", &project_endpoint],
                    ),
                    "identify GitLab source project",
                )?;
                let source_project: GitLabProject = serde_json::from_slice(&output)
                    .context("invalid JSON from glab while identifying source project")?;
                let endpoint = format!("{project_endpoint}/merge_requests");
                let output = run_command(
                    command(
                        "glab",
                        &[
                            "api",
                            "--hostname",
                            "gitlab.com",
                            "--method",
                            "GET",
                            &endpoint,
                            "-f",
                            "state=opened",
                            "-f",
                            &format!("source_branch={source_branch}"),
                            "-f",
                            "per_page=2",
                        ],
                    ),
                    "find open GitLab merge request",
                )?;
                let merge_requests: Vec<GitLabMergeRequest> = serde_json::from_slice(&output)
                    .context("invalid JSON from glab while finding merge request")?;
                merge_requests
                    .into_iter()
                    .filter(|merge_request| merge_request.source_project_id == source_project.id)
                    .map(HostPullRequest::GitLab)
                    .collect()
            }
        };
        match pull_requests.as_slice() {
            [] => Ok(None),
            [_] => Ok(pull_requests.into_iter().next()),
            _ => bail!("multiple open pull requests found for source branch '{source_branch}'"),
        }
    }
}

fn remote_parts(remote: &str) -> Result<(String, String)> {
    if remote.contains(char::is_whitespace) || remote.contains(['?', '#']) {
        bail!("push remote URL contains unsafe information");
    }
    if remote.is_empty() {
        bail!("cannot ship without a network push remote");
    }
    let (host, path) = if let Some((scheme, rest)) = remote.split_once("://") {
        if !matches!(scheme, "http" | "https" | "ssh" | "git") {
            bail!("cannot ship without a network push remote");
        }
        let (authority, path) = rest
            .split_once('/')
            .context("cannot ship without a network push remote")?;
        let host_with_port = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        let host = host_with_port
            .split_once(':')
            .map_or(host_with_port, |(host, _)| host);
        (host, path)
    } else {
        let (authority, path) = remote
            .split_once(':')
            .context("cannot ship without a network push remote")?;
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        (host, path)
    };
    let path = path
        .trim_end_matches('/')
        .strip_suffix(".git")
        .unwrap_or(path.trim_end_matches('/'));
    if host.is_empty()
        || path.is_empty()
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == ".." || !valid_path_part(part))
    {
        bail!("invalid repository path in remote URL");
    }
    Ok((host.to_ascii_lowercase(), path.to_owned()))
}

fn valid_path_part(part: &str) -> bool {
    part.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn encode_project(project: &str) -> String {
    project.replace('/', "%2F")
}

fn command(program: &str, arguments: &[&str]) -> Command {
    let mut command = Command::new(program);
    command.args(arguments).env("GH_PROMPT_DISABLED", "1");
    command
}

fn run_command(mut command: Command, action: &str) -> Result<Vec<u8>> {
    let output = command
        .output()
        .with_context(|| format!("failed to {action}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("failed to {action}: {}", stderr.trim());
    }
    Ok(output.stdout)
}

fn run_json_command(
    mut command: Command,
    payload: &serde_json::Value,
    action: &str,
) -> Result<Vec<u8>> {
    command
        .arg("--input")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .with_context(|| format!("failed to {action}"))?;
    serde_json::to_writer(
        child
            .stdin
            .as_mut()
            .context("failed to open code host command input")?,
        payload,
    )
    .with_context(|| format!("failed to send request while attempting to {action}"))?;
    drop(
        child
            .stdin
            .take()
            .context("failed to close code host command input")?,
    );
    let output = child
        .wait_with_output()
        .with_context(|| format!("failed to wait while attempting to {action}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("failed to {action}: {}", stderr.trim());
    }
    Ok(output.stdout)
}
