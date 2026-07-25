use anyhow::Result;

use crate::{git::Git, session::Session};

pub(crate) fn run(git: &Git, from: Option<&str>) -> Result<()> {
    Session::prepare()?;
    let change = git.create_change(from)?;
    let path = change.workspace();
    eprintln!("✓ Created {} at {}", change.id, path.display());
    Session::for_workspace(&path)?.attach()
}
