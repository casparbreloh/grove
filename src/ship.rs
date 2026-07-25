use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::json;

use crate::{
    git::Git,
    session::{self, Session},
};

pub(crate) fn run(git: &Git) -> Result<()> {
    let selected = git
        .current_change()?
        .context("current workspace is not a managed Grove Change")?;
    let title = selected
        .title
        .as_deref()
        .context("cannot ship an Untitled Change")?;
    let branch = publication_branch(title)?;
    let session = Session::for_workspace(&selected.path)?;
    let _lock = session.lock()?;
    let target = git.ship_target(&selected.path)?;
    let host = Host::from_remote(&target.url)?;
    let base = host.preflight()?;
    let base_ref = git.fetch_ship_base(&selected.path, &target.remote, &base)?;
    let existing = host.find_open(&branch)?;
    let published =
        existing.is_some() || git.remote_branch_exists(&selected.path, &target.remote, &branch)?;
    if !published && !git.has_ship_work(&selected.path, &base_ref)? {
        bail!("Change has no work to ship");
    }

    git.prepare_ship(&selected.path, &branch)?;
    let context = git.ship_context(&selected.path, &base_ref)?;
    let current_review = existing.as_ref().map_or_else(
        || "There is no open pull request.".to_owned(),
        |review| {
            format!(
                "Current pull request title: {}\nCurrent pull request body: {}",
                review.title, review.body
            )
        },
    );
    let prompt = format!(
        "Change title: {title}\nPublication branch: {branch}\nTarget branch: {}\nPublished history: {published}\nNew staged work: {}\n{current_review}\n\n{}",
        base, context.staged, context.text
    );
    let output = session.ship(&prompt)?;
    validate_ship_state(&output, published, context.staged, existing.is_some())?;

    git.validate_ship_snapshot(&selected.path, &branch, &context)?;
    if context.staged {
        let subject = if published {
            output.commit.as_deref().context(
                "shipping metadata did not include a commit for newly staged published work",
            )?
        } else {
            &output
                .pull_request
                .as_ref()
                .context("shipping metadata did not include pull request metadata")?
                .title
        };
        git.commit_ship(&selected.path, subject, &context)?;
    }
    git.validate_clean_ship(&selected.path)?;
    git.push_ship(&selected.path, &target.remote, &branch)?;

    let review = match existing {
        Some(existing) => {
            if let Some(metadata) = output.pull_request {
                let latest = host
                    .find_open(&branch)?
                    .context("pull request disappeared while shipping")?;
                if latest != existing {
                    bail!("pull request changed while shipping; rerun grove ship");
                }
                if latest.title == metadata.title && latest.body == metadata.body {
                    latest
                } else {
                    host.update(&branch, &latest, &metadata.title, &metadata.body)?
                }
            } else {
                existing
            }
        }
        None => {
            let metadata = output
                .pull_request
                .context("shipping metadata did not include pull request metadata")?;
            host.create(&branch, &base, &metadata.title, &metadata.body)?
        }
    };
    git.validate_clean_ship(&selected.path)?;
    println!("✓ Shipped {}", review.url);
    Ok(())
}

fn validate_ship_state(
    output: &session::ShipOutput,
    published: bool,
    staged: bool,
    has_review: bool,
) -> Result<()> {
    if !published && output.commit.is_some() {
        bail!("shipping metadata included an unnecessary initial commit subject");
    }
    if published && staged && output.commit.is_none() {
        bail!("shipping metadata did not include a commit for newly staged published work");
    }
    if !staged && output.commit.is_some() {
        bail!("shipping metadata included a commit without newly staged work");
    }
    if !has_review && output.pull_request.is_none() {
        bail!("shipping metadata did not include pull request metadata");
    }
    Ok(())
}

fn publication_branch(title: &str) -> Result<String> {
    let mut branch = String::new();
    let mut separator = false;
    for character in title.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if separator && !branch.is_empty() {
                branch.push('-');
            }
            branch.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    if branch.is_empty() {
        bail!("Change Title cannot form a publication branch");
    }
    Ok(branch)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Review {
    url: String,
    title: String,
    body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Host {
    GitHub { repository: String, owner: String },
    GitLab { project: String },
}

#[derive(Deserialize)]
struct GitHubReview {
    number: u64,
    html_url: String,
    title: String,
    body: Option<String>,
}

#[derive(Deserialize)]
struct GitLabReview {
    iid: u64,
    web_url: String,
    title: String,
    description: Option<String>,
    source_project_id: u64,
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

enum OpenReview {
    GitHub(GitHubReview),
    GitLab(GitLabReview),
}

impl OpenReview {
    fn review(&self) -> Review {
        match self {
            Self::GitHub(review) => Review {
                url: review.html_url.clone(),
                title: review.title.clone(),
                body: review.body.clone().unwrap_or_default(),
            },
            Self::GitLab(review) => Review {
                url: review.web_url.clone(),
                title: review.title.clone(),
                body: review.description.clone().unwrap_or_default(),
            },
        }
    }
}

impl Host {
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
                run_command(command("gh", &["--version"]), "check for gh")?;
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
                run_command(command("glab", &["--version"]), "check for glab")?;
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

    fn find_open(&self, source_branch: &str) -> Result<Option<Review>> {
        self.find_open_review(source_branch)?
            .map(|review| self.validate_review(review.review()))
            .transpose()
    }

    fn create(
        &self,
        source_branch: &str,
        target_branch: &str,
        title: &str,
        body: &str,
    ) -> Result<Review> {
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
                let review: GitHubReview = serde_json::from_slice(&output)
                    .context("invalid JSON from gh while creating pull request")?;
                self.validate_review(OpenReview::GitHub(review).review())
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
                let review: GitLabReview = serde_json::from_slice(&output)
                    .context("invalid JSON from glab while creating merge request")?;
                self.validate_review(OpenReview::GitLab(review).review())
            }
        }
    }

    fn update(
        &self,
        source_branch: &str,
        expected: &Review,
        title: &str,
        body: &str,
    ) -> Result<Review> {
        let review = self
            .find_open_review(source_branch)?
            .with_context(|| format!("no open review found for source branch '{source_branch}'"))?;
        if &review.review() != expected {
            bail!("review changed before it could be updated");
        }
        match (self, review) {
            (Self::GitHub { repository, .. }, OpenReview::GitHub(review)) => {
                let endpoint = format!("repos/{repository}/pulls/{}", review.number);
                let payload = json!({"title": title, "body": body});
                let output = run_json_command(
                    command("gh", &["api", "--method", "PATCH", &endpoint]),
                    &payload,
                    "update GitHub pull request",
                )?;
                let review: GitHubReview = serde_json::from_slice(&output)
                    .context("invalid JSON from gh while updating pull request")?;
                self.validate_review(OpenReview::GitHub(review).review())
            }
            (Self::GitLab { project }, OpenReview::GitLab(review)) => {
                let endpoint = format!(
                    "projects/{}/merge_requests/{}",
                    encode_project(project),
                    review.iid
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
                let review: GitLabReview = serde_json::from_slice(&output)
                    .context("invalid JSON from glab while updating merge request")?;
                self.validate_review(OpenReview::GitLab(review).review())
            }
            _ => unreachable!("review provider must match forge provider"),
        }
    }

    fn validate_review(&self, review: Review) -> Result<Review> {
        let expected = match self {
            Self::GitHub { repository, .. } => {
                format!("https://github.com/{repository}/pull/")
            }
            Self::GitLab { project } => {
                format!("https://gitlab.com/{project}/-/merge_requests/")
            }
        };
        if !review.url.starts_with(&expected)
            || review.url[expected.len()..].parse::<u64>().is_err()
        {
            bail!("forge returned an unexpected review URL");
        }
        Ok(review)
    }

    fn find_open_review(&self, source_branch: &str) -> Result<Option<OpenReview>> {
        let reviews: Vec<OpenReview> = match self {
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
                let reviews: Vec<GitHubReview> = serde_json::from_slice(&output)
                    .context("invalid JSON from gh while finding pull request")?;
                reviews.into_iter().map(OpenReview::GitHub).collect()
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
                let reviews: Vec<GitLabReview> = serde_json::from_slice(&output)
                    .context("invalid JSON from glab while finding merge request")?;
                reviews
                    .into_iter()
                    .filter(|review| review.source_project_id == source_project.id)
                    .map(OpenReview::GitLab)
                    .collect()
            }
        };
        match reviews.as_slice() {
            [] => Ok(None),
            [_] => Ok(reviews.into_iter().next()),
            _ => bail!("multiple open reviews found for source branch '{source_branch}'"),
        }
    }
}

fn remote_parts(remote: &str) -> Result<(String, String)> {
    if remote.is_empty() || remote.contains(char::is_whitespace) || remote.contains(['?', '#']) {
        bail!("invalid remote URL");
    }
    let (host, path) = if let Some((scheme, rest)) = remote.split_once("://") {
        if !matches!(scheme, "http" | "https" | "ssh" | "git") {
            bail!("unsupported remote URL scheme '{scheme}'");
        }
        let (authority, path) = rest
            .split_once('/')
            .context("remote URL has no repository path")?;
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
            .context("remote URL is not a supported URL or SCP form")?;
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
            .context("failed to open forge command input")?,
        payload,
    )
    .with_context(|| format!("failed to send request while attempting to {action}"))?;
    child
        .stdin
        .take()
        .context("failed to close forge command input")?
        .flush()
        .with_context(|| format!("failed to send request while attempting to {action}"))?;
    let output = child
        .wait_with_output()
        .with_context(|| format!("failed to wait while attempting to {action}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("failed to {action}: {}", stderr.trim());
    }
    Ok(output.stdout)
}
