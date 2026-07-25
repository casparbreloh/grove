pub(crate) const TITLE: &str = "Create a concise title of exactly three or four words for the user's request. Output only the title on one line, with no quotes, punctuation-only words, explanation, or prefix.";

pub(crate) const SHIP: &str = r#"Ship this Change as a pull request. Validate access and inspect existing branch and pull request state. Commit all work using Conventional Commit subjects without bodies, create or reuse a branch, push it, and create or update the pull request. Use a simple title and short description.

On success, summarize the result and end with `GROVE_PULL_REQUEST=<url>`. On failure, explain what completed and do not output that line."#;
