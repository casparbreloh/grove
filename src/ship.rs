use anyhow::{Context, Result, bail};

use crate::{
    git::Git,
    hosting,
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
    let forge = hosting::Forge::from_remote(&target.url)?;
    let base = forge.preflight()?;
    let base_ref = git.fetch_ship_base(&selected.path, &target.remote, &base)?;
    let existing = forge.find_open(&branch)?;
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
                let latest = forge
                    .find_open(&branch)?
                    .context("pull request disappeared while shipping")?;
                if latest != existing {
                    bail!("pull request changed while shipping; rerun grove ship");
                }
                if latest.title == metadata.title && latest.body == metadata.body {
                    latest
                } else {
                    forge.update(&branch, &latest, &metadata.title, &metadata.body)?
                }
            } else {
                existing
            }
        }
        None => {
            let metadata = output
                .pull_request
                .context("shipping metadata did not include pull request metadata")?;
            forge.create(&branch, &base, &metadata.title, &metadata.body)?
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
