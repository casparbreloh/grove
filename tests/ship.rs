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
        .env("GROVE_TEST_WORKER_RECOVERS", "1")
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"fix: allow shipping while Pi is open","body":"Allows explicit shipping without weakening activity protection for sync and archive."}}"#,
        )
        .output()
        .unwrap();
    assert!(shipped.status.success(), "{}", stderr(&shipped));
    assert_eq!(stdout(&shipped), "https://github.com/example/repo/pull/1\n");
    assert_eq!(
        repo.git_from(&change.path, ["log", "-1", "--format=%s"]),
        "fix: allow shipping while Pi is open"
    );
    assert_eq!(
        repo.git_from(&change.path, ["branch", "--show-current"]),
        "fix-grove-session-errors"
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
    let branch = "add-ai-native-shipping";
    let create_payload = format!(
        r#"{{"base":"main","body":"Adds deterministic Change shipping.","head":"{branch}","title":"feat: add AI-native shipping"}}"#
    );

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
        repo.change_record(change.path.parent().unwrap())["publication_branch"],
        branch
    );
    assert_eq!(
        repo.git_from(&change.path, ["log", "-1", "--format=%s"]),
        "feat: add AI-native shipping"
    );
    assert_eq!(
        repo.git_from(&change.path, ["rev-parse", &format!("origin/{branch}")]),
        published_head
    );
    assert_eq!(payloads(&repo.shipping_log()), [create_payload.as_str()]);
    let invocation = &repo.agent_log()[agent_before.len()..];
    for expected in [
        "mode=json",
        "arg=<--structured-output-schema>",
        "arg=<read,structured_output>",
        "A pull-request title is also a single Conventional Commit subject",
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
    assert_eq!(stdout(&rerun), "https://github.com/example/repo/pull/1\n");
    assert_eq!(repo.change_head(&change), published_head);
    assert_eq!(
        payloads(&repo.shipping_log()),
        [create_payload.as_str(), create_payload.as_str()]
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

    let published_tip = repo.change_head(&change);
    repo.git_from(&change.path, ["switch", "--detach"]);
    let rerun = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env(
            "GROVE_TEST_REVIEW_URL",
            "https://github.com/example/repo/pull/1",
        )
        .env("GROVE_TEST_REVIEW_TITLE", "feat: refine AI-native shipping")
        .env(
            "GROVE_TEST_REVIEW_BODY",
            "Adds deterministic initial and incremental shipping.",
        )
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":null}"#,
        )
        .output()
        .unwrap();
    assert!(rerun.status.success(), "{}", stderr(&rerun));
    assert_eq!(
        repo.git_from(&change.path, ["branch", "--show-current"]),
        branch
    );
    repo.git_from(&change.path, ["switch", "--detach"]);
    let detached_tip = repo.commit_file(
        &change.path,
        "detached-followup.txt",
        "detached follow-up\n",
    );
    repo.commit_file(&remote, "upstream.txt", "upstream\n");
    repo.git_from(&remote, ["push", "origin", "main"]);
    let synced = repo
        .grove()
        .arg("sync")
        .env(
            "GROVE_TEST_REMOTE_PATH",
            remote.parent().unwrap().join("origin.git"),
        )
        .output()
        .unwrap();
    assert!(synced.status.success(), "{}", stderr(&synced));
    assert!(
        stderr(&synced).contains("run sync from the Change to rebase"),
        "{}",
        stderr(&synced)
    );
    let targeted = repo.grove_from(&change.path).arg("sync").output().unwrap();
    assert!(targeted.status.success(), "{}", stderr(&targeted));
    assert!(
        stderr(&targeted).contains("published history is not rewritten"),
        "{}",
        stderr(&targeted)
    );
    assert_eq!(repo.change_head(&change), detached_tip);

    let shipped = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env(
            "GROVE_TEST_REVIEW_URL",
            "https://github.com/example/repo/pull/1",
        )
        .env("GROVE_TEST_REVIEW_TITLE", "feat: refine AI-native shipping")
        .env(
            "GROVE_TEST_REVIEW_BODY",
            "Adds deterministic initial and incremental shipping.",
        )
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":null}"#,
        )
        .output()
        .unwrap();
    assert!(shipped.status.success(), "{}", stderr(&shipped));
    assert_eq!(
        repo.git_from(&change.path, ["branch", "--show-current"]),
        branch
    );
    assert_eq!(repo.git_from(&remote, ["rev-parse", branch]), detached_tip);
    assert_ne!(published_tip, detached_tip);
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
    assert!(repo.change_record(change.path.parent().unwrap())["publication_branch"].is_null());
}

#[test]
fn duplicate_titles_have_distinct_publication_branches() {
    let repo = TestRepo::new();
    let remote = repo.create_local_origin();
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let first = repo.create_change(None);
    repo.set_change_title(&first, "Shared Publication Title");
    let first_branch = "shared-publication-title";
    repo.git_from(&first.path, ["switch", "-c", first_branch]);

    let second = repo.create_change(None);
    repo.set_change_title(&second, "Shared Publication Title");
    let expected_branch = format!("shared-publication-title-{}", second.id);
    let second_head = repo.change_head(&second);
    repo.git(["branch", &expected_branch, &second_head]);
    fs::write(second.path.join("second.txt"), "second\n").unwrap();
    let output = repo
        .grove_from(&second.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"feat: ship duplicate title","body":"Ships the second Change independently."}}"#,
        )
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let second_branch = repo.git_from(&second.path, ["branch", "--show-current"]);
    assert_eq!(second_branch, expected_branch);
    assert_ne!(second_branch, first_branch);
}

#[test]
fn ship_never_resets_or_races_an_existing_publication_branch() {
    let repo = TestRepo::new();
    let remote = repo.create_local_origin();
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Protect Existing Publication Branch");
    let base = "protect-existing-publication-branch";
    let fallback = format!("{base}-{}", change.id);
    let change_head = repo.change_head(&change);
    repo.git(["branch", base, &change_head]);
    let occupied_head = repo.commit_file(repo.path(), "occupied.txt", "occupied\n");
    repo.git(["branch", &fallback, &occupied_head]);
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    fs::write(change.path.join("change.txt"), "change\n").unwrap();
    let agent_before = repo.agent_log();

    let output = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("refusing to reset it"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        repo.git(["rev-parse", &format!("refs/heads/{fallback}")]),
        occupied_head
    );
    assert_eq!(repo.change_head(&change), change_head);
    assert_eq!(repo.agent_log(), agent_before);

    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Protect Publication Race");
    fs::write(change.path.join("race.txt"), "change\n").unwrap();
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let gate = repo.block_worker();
    let mut command = repo.grove_process_from(&change.path);
    command
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &publisher)
        .env("GROVE_TEST_WORKER_BLOCK", &gate)
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"feat: protect publication race","body":"Protects branch creation with an exact lease."}}"#,
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().unwrap();
    repo.wait_for_agent_log("prompt=");
    repo.git_from(&publisher, ["branch", "protect-publication-race", "main"]);
    repo.git_from(&publisher, ["push", "origin", "protect-publication-race"]);
    let occupied_tip = repo.git_from(&publisher, ["rev-parse", "protect-publication-race"]);
    repo.release_worker(&gate);
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(
        repo.git_from(&publisher, ["rev-parse", "protect-publication-race"]),
        occupied_tip
    );
    assert!(!repo.shipping_log().contains("--method POST"));
    let retry = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &publisher)
        .output()
        .unwrap();
    assert!(!retry.status.success(), "{retry:?}");
    assert!(
        stderr(&retry).contains("appeared before this Change published it"),
        "{retry:?}"
    );
}

#[test]
fn remote_publication_branch_collision_uses_change_id_suffix() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    repo.git_from(
        &publisher,
        ["branch", "remote-publication-collision", "main"],
    );
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Remote Publication Collision");
    fs::write(change.path.join("remote.txt"), "second\n").unwrap();

    let output = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &publisher)
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"feat: avoid remote branch collision","body":"Publishes on a Change-specific branch."}}"#,
        )
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        repo.git_from(&change.path, ["branch", "--show-current"]),
        format!("remote-publication-collision-{}", change.id)
    );
}

#[test]
fn existing_pull_request_target_overrides_default_and_is_concurrency_state() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    repo.git_from(&publisher, ["switch", "-c", "release"]);
    repo.commit_file(&publisher, "release.txt", "release\n");
    repo.git_from(&publisher, ["push", "origin", "release"]);
    repo.git([
        "remote",
        "set-url",
        "origin",
        "git@github.com:example/repo.git",
    ]);
    let change = repo.create_change(None);
    repo.set_change_title(&change, "Respect Existing Pull Request Target");
    repo.git_from(
        &change.path,
        ["switch", "-c", "respect-existing-pull-request-target"],
    );
    fs::write(change.path.join("target.txt"), "target\n").unwrap();
    let agent_before = repo.agent_log();

    let output = repo
        .grove_from(&change.path)
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &publisher)
        .env("GROVE_TEST_DEFAULT_BRANCH", "missing-default")
        .env(
            "GROVE_TEST_REVIEW_URL",
            "https://github.com/example/repo/pull/1",
        )
        .env("GROVE_TEST_REVIEW_TITLE", "feat: existing title")
        .env("GROVE_TEST_REVIEW_BODY", "Existing body.")
        .env("GROVE_TEST_REVIEW_TARGET", "release")
        .env("GROVE_TEST_RECHECK_TARGET", "main")
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":"fix: respect existing target","pull_request":{"title":"feat: updated title","body":"Updated body."}}"#,
        )
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("pull request changed during shipping"),
        "{}",
        stderr(&output)
    );
    let invocation = repo.agent_log();
    assert!(
        invocation[agent_before.len()..].contains("Target branch: release"),
        "{}",
        &invocation[agent_before.len()..]
    );
    assert!(
        !repo.shipping_log().contains("--method PATCH"),
        "{}",
        repo.shipping_log()
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
fn non_conventional_pull_request_title_does_not_commit_or_push() {
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
            r#"{"commit":null,"pull_request":{"title":"Malformed pull request title","body":"This title is not a Conventional Commit subject."}}"#,
        )
        .output()
        .unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("invalid pull request title"),
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
    let gate = repo.block_worker();
    let mut command = repo.grove_process_from(&change.path);
    command
        .arg("ship")
        .env("GROVE_TEST_REMOTE_PATH", &remote)
        .env("GROVE_TEST_WORKER_BLOCK", &gate)
        .env(
            "GROVE_TEST_SHIP_OUTPUT",
            r#"{"commit":null,"pull_request":{"title":"feat: protect shipping snapshot","body":"Publishes reviewed work only."}}"#,
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().unwrap();
    repo.wait_for_agent_log("prompt=");
    fs::write(change.path.join("feature.txt"), "changed while naming\n").unwrap();
    repo.release_worker(&gate);
    let output = child.wait_with_output().unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("Git state changed while shipping"),
        "{output:?}"
    );
    assert_eq!(repo.change_head(&change), head_before);
    assert_eq!(
        repo.git_from(&remote, ["branch", "--list", "protect-shipping-snapshot"],),
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
        "https://gitlab.com/example/repo/-/merge_requests/1\n"
    );
    let shipping = repo.shipping_log();
    assert!(shipping.contains("program=glab"), "{shipping}");
    let expected = r#"{"description":"Adds deterministic GitLab shipping.","source_branch":"support-gitlab-shipping","target_branch":"main","title":"feat: support GitLab shipping"}"#;
    assert_eq!(payloads(&shipping).last().copied(), Some(expected));
}

fn payloads(log: &str) -> Vec<&str> {
    log.lines()
        .filter_map(|line| line.strip_prefix("payload="))
        .collect()
}
