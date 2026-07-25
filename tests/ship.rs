mod support;

use std::{fs, process::Stdio};

use support::{TestChange, TestRepo, stderr, stdout};

#[test]
fn ship_runs_while_managed_pi_is_open() {
    let repo = TestRepo::new();
    let remote = repo.create_local_origin();
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let (child, gate) = repo.start_blocking_new();
    let capsule = repo.change_capsules().pop().unwrap();
    let change = TestChange {
        id: capsule.file_name().unwrap().to_string_lossy().into_owned(),
        path: capsule.join("workspace"),
    };
    repo.set_change_title(&change, "Fix Grove Session Errors");
    fs::write(change.path.join("session-fixes.txt"), "fixed\n").unwrap();

    let shipped = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"fix: allow shipping while Pi is open","body":"Allows explicit shipping without weakening activity protection for sync and archive."}}"#,
        )
        .output()
        .unwrap();
    assert!(shipped.status.success(), "{}", stderr(&shipped));
    assert_eq!(
        stdout(&shipped),
        "✓ Shipped https://github.com/example/repo/pull/1\n"
    );

    repo.release_blocking_agent(child, &gate);
}

#[test]
fn ship_recovers_after_create_failure_and_updates_pull_request_metadata() {
    let repo = TestRepo::new();
    let remote = repo.create_local_origin();
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Add AI Native Shipping");
    fs::write(change.path.join("feature.txt"), "first\n").unwrap();
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let agent_before = repo.agent_log();

    let initial_metadata = r#"{"commit":null,"pull_request":{"title":"feat: add AI-native shipping","body":"Adds deterministic Change shipping."}}"#;
    let failed = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env("GROVE_TEST_SHIP_OUTPUT", initial_metadata)
        .env("GROVE_TEST_CREATE_EXIT", "23")
        .output()
        .unwrap();
    assert!(!failed.status.success(), "{failed:?}");
    assert!(
        stderr(&failed).contains("failed to create GitHub pull request"),
        "{}",
        stderr(&failed)
    );
    let published_head = repo.change_head(&change);
    assert_eq!(
        repo.git_from(&change.path, ["log", "-1", "--format=%s"]),
        "feat: add AI-native shipping"
    );
    assert_eq!(
        repo.git_from(&change.path, ["rev-parse", "origin/add-ai-native-shipping"]),
        published_head
    );
    assert_eq!(
        payloads(&repo.shipping_log()),
        [
            r#"{"base":"main","body":"Adds deterministic Change shipping.","head":"add-ai-native-shipping","title":"feat: add AI-native shipping"}"#
        ]
    );
    let invocation = &repo.agent_log()[agent_before.len()..];
    for expected in [
        "mode=rpc",
        "arg=<--structured-output-schema>",
        "arg=<read,structured_output>",
        "Change title: Add AI Native Shipping",
        "feature.txt",
    ] {
        assert!(invocation.contains(expected), "{invocation}");
    }

    let rerun = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env("GROVE_TEST_SHIP_OUTPUT", initial_metadata)
        .env("GROVE_TEST_RESULT_TITLE", "feat: add AI-native shipping")
        .env(
            "GROVE_TEST_RESULT_BODY",
            "Adds deterministic Change shipping.",
        )
        .output()
        .unwrap();
    assert!(rerun.status.success(), "{}", stderr(&rerun));
    assert_eq!(
        stdout(&rerun),
        "✓ Shipped https://github.com/example/repo/pull/1\n"
    );
    assert_eq!(repo.change_head(&change), published_head);
    assert_eq!(
        payloads(&repo.shipping_log()),
        [
            r#"{"base":"main","body":"Adds deterministic Change shipping.","head":"add-ai-native-shipping","title":"feat: add AI-native shipping"}"#,
            r#"{"base":"main","body":"Adds deterministic Change shipping.","head":"add-ai-native-shipping","title":"feat: add AI-native shipping"}"#,
        ]
    );

    fs::write(change.path.join("feature.txt"), "first\nsecond\n").unwrap();
    let updated = repo
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
            r#"{"commit":"fix: include incremental work","pull_request":{"title":"feat: refine AI-native shipping","body":"Adds deterministic initial and incremental shipping."}}"#,
        )
        .env("GROVE_TEST_RESULT_TITLE", "feat: refine AI-native shipping")
        .env(
            "GROVE_TEST_RESULT_BODY",
            "Adds deterministic initial and incremental shipping.",
        )
        .output()
        .unwrap();
    assert!(updated.status.success(), "{}", stderr(&updated));
    assert_eq!(
        repo.git_from(&change.path, ["log", "-1", "--format=%s"]),
        "fix: include incremental work"
    );
    assert_eq!(
        payloads(&repo.shipping_log()).last().copied(),
        Some(
            r#"{"body":"Adds deterministic initial and incremental shipping.","title":"feat: refine AI-native shipping"}"#
        )
    );
    assert!(
        repo.shipping_log()
            .contains("--method PATCH repos/example/repo/pulls/1"),
        "{}",
        repo.shipping_log()
    );
}

#[test]
fn unchanged_change_stays_unpublished_when_remote_main_advances() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Leave Empty Change Unpublished");
    repo.commit_file(&publisher, "upstream.txt", "remote advance\n");
    repo.git_from(&publisher, ["push", "origin", "main"]);
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let agent_before = repo.agent_log();

    let output = repo
        .grove_from(&change.path)
        .arg("ship")
        .env(
            "GROVE_TEST_REMOTE_PATH",
            publisher.parent().unwrap().join("origin.git"),
        )
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"should not run","body":"should not run"}}"#,
        )
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(stderr(&output).contains("no work to ship"), "{output:?}");
    assert_eq!(repo.agent_log(), agent_before);
    assert_eq!(
        repo.git_from(&change.path, ["branch", "--show-current"]),
        ""
    );
    assert!(!repo.branch_exists("leave-empty-change-unpublished"));
}

#[test]
fn duplicate_title_publication_branch_is_rejected_before_pi() {
    let repo = TestRepo::new();
    let remote = repo.create_local_origin();
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let first = repo.create_change(None);
    repo.git_from(&first.path, ["switch", "-c", "shared-publication-title"]);
    let branch_head = repo.change_head(&first);

    let second = repo.create_change(None);
    repo.set_change_title(&second, "Shared Publication Title");
    fs::write(second.path.join("second.txt"), "second\n").unwrap();
    let agent_before = repo.agent_log();
    let output = repo
        .grove_from(&second.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":"feat: second change","pull_request":null}"#,
        )
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("publication branch 'shared-publication-title'"),
        "{output:?}"
    );
    assert_eq!(repo.agent_log(), agent_before);
    assert_eq!(
        repo.git_from(&second.path, ["branch", "--show-current"]),
        ""
    );
    assert_eq!(
        repo.git_from(&first.path, ["rev-parse", "shared-publication-title"]),
        branch_head
    );
}

#[test]
fn ship_rejects_invalid_remotes_before_pi() {
    let repo = TestRepo::new();
    repo.create_local_origin();
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Reject Invalid Shipping Remote");
    fs::write(change.path.join("feature.txt"), "uncommitted\n").unwrap();
    let agent_before = repo.agent_log();

    let local = repo.grove_from(&change.path).arg("ship").output().unwrap();
    assert!(!local.status.success());
    assert!(stderr(&local).contains("network push remote"));

    for (url, message) in [
        (
            "https://github.com/example/repo.git?token=secret",
            "unsafe information",
        ),
        ("git@example.com:example/repo.git", "not supported"),
    ] {
        repo.git(["remote", "set-url", "origin", url]);
        let output = repo.grove_from(&change.path).arg("ship").output().unwrap();
        assert!(!output.status.success(), "{output:?}");
        assert!(stderr(&output).contains(message), "{output:?}");
    }
    assert_eq!(repo.agent_log(), agent_before);
}

#[test]
fn malformed_structured_metadata_does_not_commit_or_push() {
    let repo = TestRepo::new();
    let remote = repo.create_local_origin();
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Reject Malformed Shipping Metadata");
    fs::write(change.path.join("feature.txt"), "uncommitted\n").unwrap();
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let head_before = repo.change_head(&change);

    let output = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"feat: malformed","body":17}}"#,
        )
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("invalid structured shipping metadata"),
        "{output:?}"
    );
    assert_eq!(repo.change_head(&change), head_before);
    assert_eq!(
        repo.git_from(
            &remote,
            ["branch", "--list", "reject-malformed-shipping-metadata"]
        ),
        ""
    );
}

#[test]
fn ship_refuses_workspace_changes_made_during_metadata_generation() {
    let repo = TestRepo::new();
    let remote = repo.create_local_origin();
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Protect Shipping Snapshot");
    fs::write(change.path.join("feature.txt"), "reviewed\n").unwrap();
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let head_before = repo.change_head(&change);
    let gate = repo.block_rpc_worker();
    let mut command = repo.grove_process_from(&change.path);
    command
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env("GROVE_TEST_RPC_BLOCK", &gate)
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"feat: protect shipping snapshot","body":"Publishes reviewed work only."}}"#,
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().unwrap();
    repo.wait_for_agent_log("rpc=");
    fs::write(change.path.join("feature.txt"), "changed while naming\n").unwrap();
    repo.release_rpc_worker(&gate);
    let output = child.wait_with_output().unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("Git state changed while shipping"),
        "{output:?}"
    );
    assert_eq!(repo.change_head(&change), head_before);
    assert_eq!(
        repo.git_from(&remote, ["branch", "--list", "protect-shipping-snapshot"]),
        ""
    );
}

#[test]
fn ship_uses_the_same_metadata_contract_for_gitlab() {
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
    let shipping = repo.shipping_log();
    assert!(shipping.contains("program=glab"), "{shipping}");
    assert_eq!(
        payloads(&shipping).last().copied(),
        Some(
            r#"{"description":"Adds deterministic GitLab shipping.","source_branch":"support-gitlab-shipping","target_branch":"main","title":"feat: support GitLab shipping"}"#
        )
    );
}

fn payloads(log: &str) -> Vec<&str> {
    log.lines()
        .filter_map(|line| line.strip_prefix("payload="))
        .collect()
}
