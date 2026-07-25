pub(crate) const CHANGE_TITLE: &str = "Create a concise title of exactly three or four words for the user's request. Output only the title on one line, with no quotes, punctuation-only words, explanation, or prefix.";

pub(crate) fn ship(remote: &str, url: &str) -> String {
    format!(
        "Ship this Change through Git remote {remote} ({url}). Detect the hosting provider from the remote and use gh or glab when available. Before changing anything, validate access and inspect existing branch and pull request state. Commit all work using Conventional Commit subjects without bodies, create or reuse a branch without rewriting published history, push it with an upstream, and create or update the pull request with a simple title and short description.\n\nOn success, summarize the result and end with `GROVE_PULL_REQUEST=<url>`. On failure, explain what completed and do not output that line."
    )
}

#[cfg(test)]
mod tests {
    use super::ship;

    #[test]
    fn shipping_prompt_includes_the_target_and_small_publication_contract() {
        let prompt = ship("origin", "https://github.com/owner/repository.git");
        for instruction in [
            "remote origin",
            "https://github.com/owner/repository.git",
            "gh or glab",
            "existing branch and pull request state",
            "Conventional Commit subjects without bodies",
            "without rewriting published history",
            "simple title and short description",
            "GROVE_PULL_REQUEST=<url>",
        ] {
            assert!(prompt.contains(instruction));
        }
    }
}
