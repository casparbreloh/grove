use anyhow::{Result, bail};

use crate::{change::display_text, git::Git};

pub(crate) fn run(git: &Git) -> Result<()> {
    let findings = git.diagnose()?;
    if findings.is_empty() {
        println!("✓ No problems found");
        return Ok(());
    }

    for finding in &findings {
        println!("! {}", display_text(finding));
    }
    bail!(
        "doctor found {} problem{}",
        findings.len(),
        if findings.len() == 1 { "" } else { "s" }
    )
}
