pub(crate) const CHANGE_TITLE: &str = "Create a concise title of exactly three or four words for the user's request. Output only the title on one line, with no quotes, punctuation-only words, explanation, or prefix.";

pub(crate) fn ship(remote: &str, repository: &str) -> String {
    format!(
        "Ship this Change to the GitHub repository {repository} using the Git remote {remote}. Inspect existing branch and pull request state, commit all work using Conventional Commit subjects without bodies, create or reuse a branch without rewriting published history, push it with an upstream, and use gh to create or update the pull request with a simple title and short description. Leave the worktree clean and summarize the result."
    )
}

#[cfg(test)]
mod tests {
    use super::ship;

    #[test]
    fn shipping_prompt_keeps_the_publication_contract_small() {
        let prompt = ship("upstream", "owner/repository");
        for instruction in [
            "repository owner/repository",
            "remote upstream",
            "existing branch and pull request state",
            "Conventional Commit subjects without bodies",
            "without rewriting published history",
            "simple title and short description",
            "Leave the worktree clean",
        ] {
            assert!(prompt.contains(instruction));
        }
    }
}
