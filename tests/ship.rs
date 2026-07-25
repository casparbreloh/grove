mod support;

use std::fs;

use support::{TestRepo, stderr, stdout};

#[test]
fn ship_publishes_and_updates_a_change_deterministically() {
    let repo = TestRepo::new();
    let remote = repo.create_local_origin();
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Add AI Native Shipping");

    let before = repo.agent_log();
    let output = repo.grove_from(&change.path).arg("ship").output().unwrap();
    assert!(!output.status.success());
    assert!(stderr(&output).contains("network push remote"));
    assert_eq!(repo.agent_log(), before);

    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let no_work = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .output()
        .unwrap();
    assert!(!no_work.status.success());
    assert!(stderr(&no_work).contains("no work to ship"));
    assert_eq!(
        repo.git_from(&change.path, ["branch", "--show-current"]),
        ""
    );
    assert_eq!(repo.agent_log(), before);

    fs::write(change.path.join("feature.txt"), "first\n").unwrap();
    let initial = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"feat: add AI-native shipping","body":"Adds deterministic Change shipping."}}"#,
        )
        .env("GROVE_TEST_RESULT_TITLE", "feat: add AI-native shipping")
        .env("GROVE_TEST_RESULT_BODY", "Adds deterministic Change shipping.")
        .output()
        .unwrap();
    assert!(initial.status.success(), "{}", stderr(&initial));
    assert_eq!(
        stdout(&initial),
        "✓ Shipped https://github.com/example/repo/pull/1\n"
    );
    assert_eq!(
        repo.git_from(&change.path, ["branch", "--show-current"]),
        "add-ai-native-shipping"
    );
    assert_eq!(
        repo.git_from(&change.path, ["log", "-1", "--format=%s"]),
        "feat: add AI-native shipping"
    );
    assert_eq!(
        repo.git_from(&change.path, ["rev-parse", "origin/add-ai-native-shipping"]),
        repo.git_from(&change.path, ["rev-parse", "HEAD"])
    );
    let invocation = &repo.agent_log()[before.len()..];
    for expected in [
        "mode=rpc",
        "arg=<--structured-output-schema>",
        "arg=<read,structured_output>",
        "Change title: Add AI Native Shipping",
        "feature.txt",
    ] {
        assert!(invocation.contains(expected), "{invocation}");
    }
    let hosting = repo.hosting_log();
    assert!(hosting.contains("auth status"), "{hosting}");
    assert!(hosting.contains("repos/example/repo/pulls"), "{hosting}");

    fs::write(change.path.join("feature.txt"), "first\nsecond\n").unwrap();
    let update = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env(
            "GROVE_TEST_REVIEW_URL",
            "https://github.com/example/repo/pull/1",
        )
        .env("GROVE_TEST_REVIEW_TITLE", "feat: add AI-native shipping")
        .env(
            "GROVE_TEST_REVIEW_BODY",
            "Adds deterministic Change shipping.",
        )
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":"fix: include incremental work","pull_request":null}"#,
        )
        .output()
        .unwrap();
    assert!(update.status.success(), "{}", stderr(&update));
    assert_eq!(
        repo.git_from(&change.path, ["log", "-1", "--format=%s"]),
        "fix: include incremental work"
    );

    let outside = repo.grove().arg("ship").output().unwrap();
    assert!(!outside.status.success());
    assert!(stderr(&outside).contains("current workspace is not a managed Grove Change"));
}

#[test]
fn ship_supports_gitlab_with_the_same_structured_worker() {
    let repo = TestRepo::new();
    let remote = repo.create_local_origin();
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Support GitLab Shipping");
    fs::write(change.path.join("gitlab.txt"), "supported\n").unwrap();
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@gitlab.com:example/repo.git",
    ]);

    let output = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"feat: support GitLab shipping","body":"Adds deterministic GitLab shipping."}}"#,
        )
        .env(
            "GROVE_TEST_RESULT_URL",
            "https://gitlab.com/example/repo/-/merge_requests/1",
        )
        .env("GROVE_TEST_RESULT_TITLE", "feat: support GitLab shipping")
        .env("GROVE_TEST_RESULT_BODY", "Adds deterministic GitLab shipping.")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "✓ Shipped https://gitlab.com/example/repo/-/merge_requests/1\n"
    );
    let hosting = repo.hosting_log();
    assert!(hosting.contains("program=glab"), "{hosting}");
    assert!(
        hosting.contains("projects/example%2Frepo/merge_requests"),
        "{hosting}"
    );
}
