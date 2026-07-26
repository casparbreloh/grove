mod support;

use std::{fs, process::Stdio};

use support::{TestRepo, stderr, stdout};

#[test]
fn sync_fetches_main_archives_integrated_and_rebases_every_other_change() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    repo.git_from(&publisher, ["switch", "-c", "unrelated"]);
    repo.commit_file(&publisher, "unrelated.txt", "first\n");
    repo.git_from(&publisher, ["push", "origin", "unrelated"]);
    repo.git(["fetch", "origin", "unrelated:refs/remotes/origin/unrelated"]);
    let unrelated_before = repo.git(["rev-parse", "refs/remotes/origin/unrelated"]);
    repo.commit_file(&publisher, "unrelated.txt", "second\n");
    repo.git_from(&publisher, ["push", "origin", "unrelated"]);
    repo.git_from(&publisher, ["switch", "main"]);

    let integrated = repo.create_change(Some("main"));
    repo.set_change_title(&integrated, "Integrated Change");
    let integrated_tip = repo.commit_file(&integrated.path, "integrated.txt", "integrated\n");
    let remaining = repo.create_change(Some("main"));
    repo.set_change_title(&remaining, "Remaining Change");
    repo.commit_file(&remaining.path, "remaining.txt", "remaining\n");
    let remaining_before = repo.change_head(&remaining);
    let malformed = repo.create_change(Some("main"));
    fs::write(
        malformed.path.parent().unwrap().join("change.json"),
        "not json\n",
    )
    .unwrap();

    repo.git_from(
        &publisher,
        ["fetch", repo.path().to_str().unwrap(), &integrated_tip],
    );
    repo.git_from(
        &publisher,
        ["merge", "--no-ff", "-m", "Integrate Change", "FETCH_HEAD"],
    );
    repo.git_from(&publisher, ["push", "origin", "main"]);
    let upstream = repo.git_from(&publisher, ["rev-parse", "main"]);

    let output = repo.grove().arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "Archived Integrated Change\nRebased Remaining Change\n"
    );
    assert_eq!(repo.git(["rev-parse", "main"]), upstream);
    assert_eq!(
        repo.git(["rev-parse", "refs/remotes/origin/main"]),
        upstream
    );
    assert_eq!(
        repo.git(["rev-parse", "refs/remotes/origin/unrelated"]),
        unrelated_before
    );
    assert!(!integrated.path.exists());
    assert_eq!(
        repo.change_record(integrated.path.parent().unwrap())["state"],
        "archived"
    );
    assert_ne!(repo.change_head(&remaining), remaining_before);
    repo.git_from(
        &remaining.path,
        ["merge-base", "--is-ancestor", &upstream, "HEAD"],
    );
    assert_eq!(
        repo.change_record(remaining.path.parent().unwrap())["state"],
        "active"
    );
    assert!(malformed.path.exists());

    let output = repo.grove().arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

#[test]
fn sync_leaves_published_changes_untouched_and_runs_only_from_main() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    let published = repo.create_change(Some("main"));
    repo.set_change_title(&published, "Shared Title");
    repo.commit_file(&published.path, "published.txt", "published\n");
    repo.git_from(&published.path, ["switch", "-c", "published-change"]);
    repo.git_from(
        &published.path,
        ["push", "--set-upstream", "origin", "published-change"],
    );
    let published_tip = repo.change_head(&published);

    let unpublished = repo.create_change(Some("main"));
    repo.set_change_title(&unpublished, "Shared Title");
    repo.commit_file(&unpublished.path, "unpublished.txt", "unpublished\n");
    let unpublished_before = repo.change_head(&unpublished);

    repo.commit_file(&publisher, "upstream.txt", "upstream\n");
    repo.git_from(&publisher, ["push", "origin", "main"]);
    let upstream = repo.git_from(&publisher, ["rev-parse", "main"]);

    let output = repo.grove().arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        format!("Rebased Shared Title · {}\n", &unpublished.id[..8])
    );
    assert_eq!(repo.change_head(&published), published_tip);
    assert_ne!(repo.change_head(&unpublished), unpublished_before);
    repo.git_from(
        &unpublished.path,
        ["merge-base", "--is-ancestor", &upstream, "HEAD"],
    );

    let output = repo
        .grove_from(&unpublished.path)
        .arg("sync")
        .output()
        .unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("grove sync must be run from the primary worktree"),
        "{output:?}"
    );
}

#[test]
fn sync_does_not_race_a_change_being_shipped() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    let change = repo.create_change(Some("main"));
    repo.set_change_title(&change, "Shipping Change");
    repo.commit_file(&change.path, "change.txt", "change\n");
    let change_head = repo.change_head(&change);
    repo.git([
        "remote",
        "set-url",
        "--push",
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
            r#"{"commit":null,"pull_request":{"title":"feat: ship safely","body":"Ships without racing synchronization."}}"#,
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().unwrap();
    repo.wait_for_agent_log("prompt=");

    repo.commit_file(&publisher, "upstream.txt", "upstream\n");
    repo.git_from(&publisher, ["push", "origin", "main"]);
    let upstream = repo.git_from(&publisher, ["rev-parse", "main"]);
    let output = repo.grove().arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(repo.git(["rev-parse", "main"]), upstream);
    assert_eq!(repo.change_head(&change), change_head);

    repo.release_worker(&gate);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn sync_reports_restored_conflicts_but_continues_rebasing_other_changes() {
    let repo = TestRepo::new();
    repo.commit_file(repo.path(), "shared.txt", "base\n");
    let publisher = repo.create_local_origin();

    let conflicting = repo.create_change(Some("main"));
    repo.set_change_title(&conflicting, "Conflicting Change");
    fs::write(conflicting.path.join("shared.txt"), "change\n").unwrap();
    repo.git_from(&conflicting.path, ["add", "shared.txt"]);
    repo.git_from(&conflicting.path, ["commit", "-m", "change shared file"]);
    let conflicting_head = repo.change_head(&conflicting);
    let conflicting_status = repo.git_from(&conflicting.path, ["status", "--porcelain=v1"]);

    let dirty = repo.create_change(Some("main"));
    repo.set_change_title(&dirty, "Dirty Change");
    repo.commit_file(&dirty.path, "committed.txt", "committed\n");
    fs::write(dirty.path.join("dirty.txt"), "dirty\n").unwrap();
    let dirty_head = repo.change_head(&dirty);

    let later = repo.create_change(Some("main"));
    repo.set_change_title(&later, "Later Change");
    repo.commit_file(&later.path, "later.txt", "later\n");
    let later_head = repo.change_head(&later);

    repo.commit_file(&publisher, "shared.txt", "primary\n");
    repo.git_from(&publisher, ["push", "origin", "main"]);
    let upstream = repo.git_from(&publisher, ["rev-parse", "main"]);

    let output = repo.grove().arg("sync").output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "Rebased Later Change\nCould not rebase Conflicting Change; restored unchanged\nError: sync encountered 1 rebase conflict\n"
    );
    assert_eq!(repo.change_head(&conflicting), conflicting_head);
    assert_eq!(
        repo.git_from(&conflicting.path, ["status", "--porcelain=v1"]),
        conflicting_status
    );
    assert_eq!(repo.change_head(&dirty), dirty_head);
    assert_eq!(
        fs::read_to_string(dirty.path.join("dirty.txt")).unwrap(),
        "dirty\n"
    );
    assert_ne!(repo.change_head(&later), later_head);
    repo.git_from(
        &later.path,
        ["merge-base", "--is-ancestor", &upstream, "HEAD"],
    );
}

#[test]
fn sync_validates_main_before_mutation() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    repo.commit_file(&publisher, "upstream.txt", "upstream\n");
    repo.git_from(&publisher, ["push", "origin", "main"]);
    let main_before = repo.git(["rev-parse", "main"]);
    let tracking_before = repo.git(["rev-parse", "refs/remotes/origin/main"]);
    fs::write(repo.path().join("dirty.txt"), "dirty\n").unwrap();

    let output = repo.grove().arg("sync").output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("primary worktree has uncommitted changes"),
        "{output:?}"
    );
    assert_eq!(repo.git(["rev-parse", "main"]), main_before);
    assert_eq!(
        repo.git(["rev-parse", "refs/remotes/origin/main"]),
        tracking_before
    );

    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    repo.commit_file(repo.path(), "local.txt", "local\n");
    let main_before = repo.git(["rev-parse", "main"]);
    repo.commit_file(&publisher, "remote.txt", "remote\n");
    repo.git_from(&publisher, ["push", "origin", "main"]);
    let remote_tip = repo.git_from(&publisher, ["rev-parse", "main"]);
    let output = repo.grove().arg("sync").output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(stderr(&output).contains("cannot be fast-forwarded"));
    assert_eq!(repo.git(["rev-parse", "main"]), main_before);
    assert_eq!(
        repo.git(["rev-parse", "refs/remotes/origin/main"]),
        remote_tip
    );
}
