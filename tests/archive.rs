mod support;

use std::{fs, path::PathBuf};

use support::{
    TestChange, TestRepo, assert_inline_terminal_restored, assert_terminal_restored, stderr, stdout,
};

#[test]
fn integrated_merge_cherry_pick_and_squash_archive_but_unmerged_work_does_not() {
    let merged = TestRepo::new();
    let change = merged.create_change(None);
    let tip = merged.commit_file(&change.path, "merged.txt", "merged\n");
    merged.git(["merge", "--no-ff", "-m", "Merge change", &tip]);
    merged
        .grove_from(&change.path)
        .arg("archive")
        .assert()
        .success();
    assert!(!change.path.exists());

    let cherry_picked = TestRepo::new();
    let change = cherry_picked.create_change(None);
    cherry_picked.git_from(&change.path, ["switch", "-c", "published-change"]);
    cherry_picked.git(["branch", "--set-upstream-to=main", "published-change"]);
    let tip = cherry_picked.commit_file(&change.path, "picked.txt", "picked\n");
    cherry_picked.git(["cherry-pick", &tip]);
    cherry_picked
        .grove_from(&change.path)
        .arg("archive")
        .assert()
        .success();
    assert!(!change.path.exists());
    assert_eq!(cherry_picked.git(["rev-parse", "published-change"]), tip);

    let squashed = TestRepo::new();
    let change = squashed.create_change(None);
    squashed.commit_file(&change.path, "one.txt", "one\n");
    let tip = squashed.commit_file(&change.path, "two.txt", "two\n");
    squashed.git(["merge", "--squash", &tip]);
    squashed.git(["commit", "-m", "Squash change"]);
    squashed
        .grove_from(&change.path)
        .arg("archive")
        .assert()
        .success();
    assert!(!change.path.exists());

    let unmerged = TestRepo::new();
    let change = unmerged.create_change(None);
    unmerged.commit_file(&change.path, "unmerged.txt", "unmerged\n");
    let error = unmerged
        .grove_from(&change.path)
        .arg("archive")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&error).contains("not merged"), "{}", stderr(&error));
    assert!(change.path.exists());
}

#[test]
fn merge_resolution_only_content_is_never_mistaken_for_integration() {
    let repo = TestRepo::new();
    let change = repo.create_change(None);
    let worktree = &change.path;
    let topic_change = repo.commit_file(worktree, "shared.txt", "shared\n");
    repo.commit_file(repo.path(), "main.txt", "main\n");
    repo.git_from(worktree, ["merge", "--no-ff", "--no-commit", "main"]);
    fs::write(worktree.join("only-in-merge.txt"), "unique resolution\n").unwrap();
    repo.git_from(worktree, ["add", "only-in-merge.txt"]);
    repo.git_from(worktree, ["commit", "-m", "Unique merge resolution"]);
    repo.git(["cherry-pick", &topic_change]);
    let cherry = repo.git(["cherry", "main", &repo.change_head(&change)]);
    assert!(
        !cherry.is_empty() && cherry.lines().all(|line| line.starts_with('-')),
        "{cherry}"
    );

    let error = repo
        .grove_from(worktree)
        .arg("archive")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&error).contains("not merged"), "{}", stderr(&error));
    assert!(worktree.exists());
}

#[test]
fn archive_picker_cancels_or_archives_the_selected_change() {
    let repo = TestRepo::new();
    let mut changes = (0..4).map(|_| repo.create_change(None)).collect::<Vec<_>>();
    changes.sort_by_key(|change| {
        (
            repo.change_record(change.path.parent().unwrap())["created_at"]
                .as_u64()
                .unwrap(),
            change.id.clone(),
        )
    });
    for (index, change) in changes.iter().enumerate() {
        repo.set_change_title(change, &format!("Archive Change {}", index + 1));
    }

    let cancelled = repo.archive_without_color_in_pty("Archive Change 1", b"\x1b");
    assert!(cancelled.status.success(), "{cancelled:?}");
    assert!(changes.iter().all(|change| change.path.exists()));
    assert!(!stdout(&cancelled).contains("\x1b[1m"), "{cancelled:?}");
    assert_terminal_restored(&stdout(&cancelled));

    let archived = repo.archive_in_short_pty("Archive Change 1", b"\x1b[B\x1b[B\x1b[B\r");
    assert!(archived.status.success(), "{archived:?}");
    assert!(stdout(&archived).contains("✓ Archived Archive Change 4"));
    assert!(changes[..3].iter().all(|change| change.path.exists()));
    assert!(!changes[3].path.exists());
    assert_terminal_restored(&stdout(&archived));
}

#[test]
fn archive_preserves_native_sessions_and_excludes_change() {
    let repo = TestRepo::new();
    let change = repo.create_change(Some("main"));
    repo.set_change_title(&change, "Archive Finished Change");
    let tip = repo.commit_file(&change.path, "finished.txt", "finished\n");
    repo.git(["merge", "--no-ff", "-m", "Merge archived change", &tip]);
    let capsule = change.path.parent().unwrap();
    let sessions = capsule.join("pi");
    fs::create_dir_all(&sessions).unwrap();
    let session = sessions.join("native.jsonl");
    let session_contents = b"{\"type\":\"session\",\"id\":\"native\"}\n";
    fs::write(&session, session_contents).unwrap();

    repo.grove_from(&change.path)
        .arg("archive")
        .assert()
        .success();
    assert_eq!(repo.navigation(), repo.path().canonicalize().unwrap());
    assert!(!change.path.exists());
    assert!(capsule.exists());
    assert_eq!(fs::read(&session).unwrap(), session_contents);
    let record = repo.change_record(capsule);
    assert_eq!(record["state"], "archived");
    assert_eq!(record["outcome"], "integrated");
    assert!(record["archived_at"].is_number());
    assert!(record.get("closing").is_none());
    let navigator = repo.navigator_without_color_in_pty(repo.path(), "Main", b"\x1b");
    assert!(navigator.status.success(), "{navigator:?}");
    assert!(!stdout(&navigator).contains("Archive Finished Change"));
    assert_inline_terminal_restored(&stdout(&navigator));
}

#[test]
fn interrupted_recovery_does_not_consume_the_requested_archive() {
    let repo = TestRepo::new();
    let interrupted = repo.create_change(None);
    let interrupted_capsule = interrupt_archive(&repo, &interrupted, true);

    let requested = repo.create_change(None);
    repo.set_change_title(&requested, "Archive Requested Change");
    let requested_tip = repo.commit_file(&requested.path, "requested.txt", "requested\n");
    repo.git(["merge", "--no-ff", "-m", "Merge requested", &requested_tip]);

    let output = repo
        .grove_from(&requested.path)
        .arg("archive")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("✓ Recovered 1 interrupted archive\n")
            && stderr(&output).contains("✓ Archived Archive Requested Change\n"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        repo.change_record(&interrupted_capsule)["state"],
        "archived"
    );
    assert!(
        !repo
            .git(["worktree", "list", "--porcelain"])
            .contains(&interrupted.path.display().to_string())
    );
    assert!(!requested.path.exists());

    let repo = TestRepo::new();
    let change = repo.create_change(None);
    let tip = repo.commit_file(&change.path, "integrated.txt", "integrated\n");
    repo.git(["merge", "--no-ff", "-m", "Integrate Change", &tip]);
    let target = repo.git(["rev-parse", "main"]);
    let capsule = mark_closing(&repo, &change);
    let mut record = repo.change_record(&capsule);
    record["closing"]["outcome"] = "integrated".into();
    record["closing"]["target_oid"] = target.into();
    record["closing"]["target_ref"] = "refs/heads/main".into();
    fs::write(
        capsule.join("change.json"),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
    repo.git([
        "worktree",
        "remove",
        "--force",
        change.path.to_str().unwrap(),
    ]);
    repo.commit_file(repo.path(), "later.txt", "later\n");
    repo.grove().arg("archive").assert().success();
    assert_eq!(repo.change_record(&capsule)["state"], "archived");
}

#[test]
fn interrupted_recovery_refuses_ambiguous_workspaces() {
    let repo = TestRepo::new();
    let change = repo.create_change(None);
    let capsule = interrupt_archive(&repo, &change, false);
    fs::create_dir(&change.path).unwrap();
    fs::write(change.path.join("reappeared.txt"), "do not delete\n").unwrap();
    let output = repo.grove().arg("archive").output().unwrap();

    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("workspace exists without a Git worktree"),
        "{}",
        stderr(&output)
    );
    assert_eq!(repo.change_record(&capsule)["state"], "closing");
    assert_eq!(
        fs::read_to_string(change.path.join("reappeared.txt")).unwrap(),
        "do not delete\n"
    );

    let repo = TestRepo::new();
    let change = repo.create_change(None);
    let capsule = mark_closing(&repo, &change);
    repo.commit_file(&change.path, "changed.txt", "changed\n");
    let output = repo.grove().arg("archive").output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("workspace HEAD changed"),
        "{output:?}"
    );
    assert_eq!(repo.change_record(&capsule)["state"], "closing");

    let repo = TestRepo::new();
    let change = repo.create_change(None);
    let capsule = mark_closing(&repo, &change);
    let before = fs::read(capsule.join("change.json")).unwrap();
    let output = repo
        .grove_from(&change.path)
        .arg("archive")
        .env_remove("GROVE_DIRECTIVE_CD_FILE")
        .output()
        .unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("shell integration is not loaded"),
        "{output:?}"
    );
    assert_eq!(fs::read(capsule.join("change.json")).unwrap(), before);

    let repo = TestRepo::new();
    let change = repo.create_change(None);
    let capsule = interrupt_archive(&repo, &change, false);
    let ordinary = repo.home().join("ordinary-archive-worktree");
    repo.git(["branch", "ordinary-archive"]);
    repo.git([
        "worktree",
        "add",
        ordinary.to_str().unwrap(),
        "ordinary-archive",
    ]);
    let output = repo.grove_from(&ordinary).arg("archive").output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(stderr(&output).contains("not a managed Grove Change"));
    assert_eq!(repo.change_record(&capsule)["state"], "closing");
}

fn interrupt_archive(repo: &TestRepo, change: &TestChange, registered: bool) -> PathBuf {
    let capsule = mark_closing(repo, change);
    if registered {
        fs::remove_dir_all(&change.path).unwrap();
    } else {
        repo.git([
            "worktree",
            "remove",
            "--force",
            change.path.to_str().unwrap(),
        ]);
    }
    capsule
}

fn mark_closing(repo: &TestRepo, change: &TestChange) -> PathBuf {
    let capsule = change.path.parent().unwrap().to_owned();
    let mut record = repo.change_record(&capsule);
    record["state"] = "closing".into();
    record["closing"] = serde_json::json!({
        "outcome": "discarded",
        "tip_oid": repo.change_head(change),
        "target_oid": null,
        "target_ref": null,
        "local_branch": null
    });
    fs::write(
        capsule.join("change.json"),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
    capsule
}

#[test]
fn force_discards_local_work_but_keeps_git_locked_worktrees_protected() {
    let repo = TestRepo::new();
    let change = repo.create_change(None);
    let capsule = change.path.parent().unwrap();
    repo.git_from(&change.path, ["switch", "-c", "discarded-change"]);
    repo.commit_file(&change.path, "committed.txt", "committed\n");
    fs::write(change.path.join("dirty.txt"), "discarded\n").unwrap();
    let head_before = repo.change_head(&change);
    let status_before = repo.git_from(&change.path, ["status", "--porcelain=v1"]);
    let dirty_before = fs::read(change.path.join("dirty.txt")).unwrap();
    repo.git([
        "worktree",
        "lock",
        "--reason",
        "protected from forced archive",
        change.path.to_str().unwrap(),
    ]);
    let record_before = fs::read(capsule.join("change.json")).unwrap();

    let locked = repo
        .grove_from(&change.path)
        .args(["archive", "--force"])
        .output()
        .unwrap();
    assert!(!locked.status.success(), "{locked:?}");
    assert!(stderr(&locked).contains("locked"), "{locked:?}");
    assert!(change.path.exists());
    assert_eq!(repo.change_head(&change), head_before);
    assert_eq!(
        repo.git_from(&change.path, ["status", "--porcelain=v1"]),
        status_before
    );
    assert_eq!(
        fs::read(change.path.join("dirty.txt")).unwrap(),
        dirty_before
    );
    assert_eq!(
        fs::read(capsule.join("change.json")).unwrap(),
        record_before
    );
    assert!(repo.branch_exists("discarded-change"));

    repo.git(["worktree", "unlock", change.path.to_str().unwrap()]);
    let rebase =
        PathBuf::from(repo.git_from(&change.path, ["rev-parse", "--git-path", "rebase-merge"]));
    fs::create_dir_all(&rebase).unwrap();
    let todo = rebase.join("git-rebase-todo");
    fs::write(&todo, "break\n").unwrap();
    let operation = repo
        .grove_from(&change.path)
        .args(["archive", "--force"])
        .output()
        .unwrap();
    assert!(!operation.status.success(), "{operation:?}");
    assert!(stderr(&operation).contains("Git operation is in progress"));
    assert_eq!(fs::read_to_string(&todo).unwrap(), "break\n");
    assert!(change.path.exists());
    fs::remove_dir_all(rebase).unwrap();

    repo.grove_from(&change.path)
        .args(["archive", "--force"])
        .assert()
        .success();

    let record = repo.change_record(capsule);
    assert_eq!(record["state"], "archived");
    assert_eq!(record["outcome"], "discarded");
    assert!(record["archived_at"].is_number());
    assert!(!change.path.exists());
    assert!(!repo.branch_exists("discarded-change"));
}
