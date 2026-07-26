mod support;

use std::{fs, path::PathBuf};

use support::{TestRepo, stderr, stdout};

#[test]
fn primary_sync_fetches_exact_upstream_archives_integrated_and_leaves_other_changes() {
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
    let remaining_tip = repo.commit_file(&remaining.path, "remaining.txt", "remaining\n");
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
    assert_sync_report(
        &output,
        &[
            ("Integrated Change", "archived", "integrated upstream"),
            (
                "Remaining Change",
                "skipped",
                "run sync from the Change to rebase",
            ),
        ],
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
    assert_eq!(repo.change_head(&remaining), remaining_tip);
    assert_eq!(
        repo.change_record(remaining.path.parent().unwrap())["state"],
        "active"
    );
    assert!(malformed.path.exists());
}

#[test]
fn change_sync_rebases_only_that_change_and_never_rewrites_published_history() {
    let repo = TestRepo::new();
    repo.create_local_origin();
    let change = repo.create_change(Some("main"));
    repo.set_change_title(&change, "Targeted Rebase");
    repo.commit_file(&change.path, "change.txt", "change\n");
    let sibling = repo.create_change(Some("main"));
    repo.set_change_title(&sibling, "Untouched Sibling");
    let sibling_tip = repo.commit_file(&sibling.path, "sibling.txt", "sibling\n");
    let remote_before = repo.git(["rev-parse", "refs/remotes/origin/main"]);

    repo.commit_file(repo.path(), "primary-one.txt", "primary one\n");
    let first_primary = repo.git(["rev-parse", "main"]);
    let output = repo.grove_from(&change.path).arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_sync_report(&output, &[("Targeted Rebase", "rebased", "onto primary")]);
    repo.git_from(
        &change.path,
        ["merge-base", "--is-ancestor", &first_primary, "HEAD"],
    );
    assert_eq!(repo.change_head(&sibling), sibling_tip);
    assert_eq!(
        repo.git(["rev-parse", "refs/remotes/origin/main"]),
        remote_before
    );

    repo.commit_file(repo.path(), "primary-two.txt", "primary two\n");
    let second_primary = repo.git(["rev-parse", "main"]);
    repo.grove_from(&change.path).arg("sync").assert().success();
    repo.git_from(
        &change.path,
        ["merge-base", "--is-ancestor", &second_primary, "HEAD"],
    );
    assert_eq!(repo.change_head(&sibling), sibling_tip);

    repo.git_from(&change.path, ["switch", "-c", "published-change"]);
    repo.git_from(
        &change.path,
        ["push", "--set-upstream", "origin", "published-change"],
    );
    let published_tip = repo.change_head(&change);
    repo.commit_file(repo.path(), "primary-three.txt", "primary three\n");
    let output = repo.grove_from(&change.path).arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_sync_report(
        &output,
        &[(
            "Targeted Rebase",
            "skipped",
            "published history is not rewritten",
        )],
    );
    assert_eq!(repo.change_head(&change), published_tip);
}

#[test]
fn change_sync_restores_conflicts_and_skips_dirty_or_invalid_changes() {
    let repo = TestRepo::new();
    repo.commit_file(repo.path(), "shared.txt", "base\n");
    let change = repo.create_change(Some("main"));
    repo.set_change_title(&change, "Conflicting Change");
    fs::write(change.path.join("shared.txt"), "change\n").unwrap();
    repo.git_from(&change.path, ["add", "shared.txt"]);
    repo.git_from(&change.path, ["commit", "-m", "change shared file"]);
    let original_head = repo.change_head(&change);
    let original_status = repo.git_from(&change.path, ["status", "--porcelain=v1"]);
    repo.commit_file(repo.path(), "shared.txt", "primary\n");

    let output = repo.grove_from(&change.path).arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_sync_report(
        &output,
        &[(
            "Conflicting Change",
            "skipped",
            "rebase failed; Change restored",
        )],
    );
    assert_eq!(repo.change_head(&change), original_head);
    assert_eq!(
        repo.git_from(&change.path, ["status", "--porcelain=v1"]),
        original_status
    );

    let rebase =
        PathBuf::from(repo.git_from(&change.path, ["rev-parse", "--git-path", "rebase-merge"]));
    fs::create_dir_all(&rebase).unwrap();
    let todo = rebase.join("git-rebase-todo");
    fs::write(&todo, "pick preserved\n").unwrap();
    let output = repo.grove_from(&change.path).arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_sync_report(
        &output,
        &[(
            "Conflicting Change",
            "skipped",
            "Git operation is in progress",
        )],
    );
    assert_eq!(fs::read_to_string(&todo).unwrap(), "pick preserved\n");
    fs::remove_dir_all(rebase).unwrap();

    fs::write(change.path.join("dirty.txt"), "dirty\n").unwrap();
    let output = repo.grove_from(&change.path).arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_sync_report(
        &output,
        &[(
            "Conflicting Change",
            "skipped",
            "worktree has uncommitted changes",
        )],
    );
    assert_eq!(repo.change_head(&change), original_head);
}

#[test]
fn primary_sync_validates_before_mutation() {
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

fn assert_sync_report(output: &std::process::Output, expected: &[(&str, &str, &str)]) {
    assert_eq!(stdout(output), "");
    let report = stderr(output);
    for (title, outcome, reason) in expected {
        let matching = report
            .lines()
            .filter(|line| {
                line.to_ascii_lowercase()
                    .contains(&title.to_ascii_lowercase())
                    && line.contains(outcome)
                    && line.contains(reason)
            })
            .count();
        assert_eq!(
            matching, 1,
            "expected one sync row for {title:?}, {outcome:?}, {reason:?}: {report}"
        );
    }
    assert!(report.contains(&format!("✓ Synced {} Changes", expected.len())));
}
