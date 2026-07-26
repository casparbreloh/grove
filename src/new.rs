use anyhow::Result;

use crate::{change::display_text, git::Git, session};

pub(crate) fn run_new(git: &Git, from: Option<&str>) -> Result<()> {
    session::require_pi_available()?;
    let change = git.create_change(from)?;
    let path = change.workspace();
    eprintln!(
        "✓ Created {} at {}",
        change.id(),
        display_text(&path.display().to_string())
    );
    session::attach_pi_session(&path)
}
