mod support;

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

use support::{TestRepo, stderr, stdout};

#[test]
fn doctor_reports_local_state_without_changing_it() {
    let repo = TestRepo::new();
    let healthy = repo.create_change(None);
    repo.set_change_title(&healthy, "Healthy Change");
    let healthy_capsule = healthy.path.parent().unwrap();

    let archived = repo.create_change(None);
    let archived_capsule = archived.path.parent().unwrap();
    repo.git([
        "worktree",
        "remove",
        "--force",
        archived.path.to_str().unwrap(),
    ]);
    let mut archived_record = repo.change_record(archived_capsule);
    archived_record["title"] = "Archived Change".into();
    archived_record["publication_branch"] = "archived-change".into();
    archived_record["published_oid"] = "0000000000000000000000000000000000000000".into();
    archived_record["base_oid"] = "0000000000000000000000000000000000000000".into();
    archived_record["state"] = "archived".into();
    archived_record["archived_at"] = 1.into();
    archived_record["outcome"] = "discarded".into();
    fs::write(
        archived_capsule.join("change.json"),
        serde_json::to_vec_pretty(&archived_record).unwrap(),
    )
    .unwrap();
    let archived_record_before = fs::read(archived_capsule.join("change.json")).unwrap();

    let metadata_lock = healthy_capsule.join(".metadata.lock");
    let mutation_lock = healthy_capsule.join(".mutation.lock");
    for lock in [&metadata_lock, &mutation_lock] {
        if lock.exists() {
            fs::remove_file(lock).unwrap();
        }
    }
    let healthy_record_before = fs::read(healthy_capsule.join("change.json")).unwrap();
    let worktrees_before = repo.git(["worktree", "list", "--porcelain"]);
    let refs_before = repo.git(["show-ref"]);

    let output = repo
        .grove()
        .arg("doctor")
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(stdout(&output).contains("No problems found"));
    assert_eq!(
        fs::read(healthy_capsule.join("change.json")).unwrap(),
        healthy_record_before
    );
    assert!(!metadata_lock.exists(), "doctor must not create lock files");
    assert!(!mutation_lock.exists(), "doctor must not create lock files");
    assert_eq!(
        fs::read(archived_capsule.join("change.json")).unwrap(),
        archived_record_before
    );
    assert_eq!(
        repo.git(["worktree", "list", "--porcelain"]),
        worktrees_before
    );
    assert_eq!(repo.git(["show-ref"]), refs_before);

    let malformed = healthy_capsule.parent().unwrap().join("deadbeef");
    fs::create_dir(&malformed).unwrap();
    fs::write(malformed.join("change.json"), b"not json\n").unwrap();

    let missing = repo.create_change(None);
    repo.set_change_title(&missing, "Missing Workspace");
    fs::remove_dir_all(&missing.path).unwrap();

    let moved = repo.create_change(None);
    repo.set_change_title(&moved, "Moved Workspace");
    let moved_path = moved.path.parent().unwrap().join("elsewhere");
    repo.git([
        "worktree",
        "move",
        moved.path.to_str().unwrap(),
        moved_path.to_str().unwrap(),
    ]);

    let symlinked = repo.create_change(None);
    repo.set_change_title(&symlinked, "Symlinked Workspace");
    let symlink_target = symlinked.path.parent().unwrap().join("actual-workspace");
    fs::rename(&symlinked.path, &symlink_target).unwrap();
    symlink(&symlink_target, &symlinked.path).unwrap();

    let archived_misplaced = repo.create_change(None);
    repo.set_change_title(&archived_misplaced, "Archived Misplaced");
    let archived_misplaced_capsule = archived_misplaced.path.parent().unwrap();
    let archived_misplaced_path = archived_misplaced_capsule.join("elsewhere");
    repo.git([
        "worktree",
        "move",
        archived_misplaced.path.to_str().unwrap(),
        archived_misplaced_path.to_str().unwrap(),
    ]);
    let mut archived_misplaced_record = repo.change_record(archived_misplaced_capsule);
    archived_misplaced_record["state"] = "archived".into();
    archived_misplaced_record["archived_at"] = 1.into();
    archived_misplaced_record["outcome"] = "discarded".into();
    fs::write(
        archived_misplaced_capsule.join("change.json"),
        serde_json::to_vec_pretty(&archived_misplaced_record).unwrap(),
    )
    .unwrap();

    let closing = repo.create_change(None);
    let closing_capsule = closing.path.parent().unwrap();
    let mut closing_record = repo.change_record(closing_capsule);
    closing_record["state"] = "closing".into();
    closing_record["closing"] = serde_json::json!({
        "outcome": "discarded",
        "tip_oid": repo.change_head(&closing),
        "target_oid": null,
        "target_ref": null,
        "local_branch": null
    });
    fs::write(
        closing_capsule.join("change.json"),
        serde_json::to_vec_pretty(&closing_record).unwrap(),
    )
    .unwrap();

    let publication = repo.create_change(None);
    let publication_capsule = publication.path.parent().unwrap();
    let mut publication_record = repo.change_record(publication_capsule);
    publication_record["title"] = "Expected Branch".into();
    publication_record["publication_branch"] = "wrong-branch".into();
    publication_record["published_oid"] = "deadbeef".into();
    fs::write(
        publication_capsule.join("change.json"),
        serde_json::to_vec_pretty(&publication_record).unwrap(),
    )
    .unwrap();

    let mut permissions = fs::metadata(publication_capsule).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(publication_capsule, permissions).unwrap();
    fs::create_dir(publication_capsule.join(".metadata.lock")).unwrap();
    fs::create_dir(publication_capsule.join(".mutation.lock")).unwrap();

    let records_before = [
        malformed.join("change.json"),
        missing.path.parent().unwrap().join("change.json"),
        closing_capsule.join("change.json"),
        publication_capsule.join("change.json"),
    ]
    .map(|path| (path.clone(), fs::read(path).unwrap()));
    let worktrees_before = repo.git(["worktree", "list", "--porcelain"]);
    let refs_before = repo.git(["show-ref"]);

    let output = repo
        .grove()
        .arg("doctor")
        .assert()
        .failure()
        .get_output()
        .clone();
    let report = stdout(&output);

    for expected in [
        "deadbeef: invalid change record",
        "Missing Workspace ·",
        "worktree registration is stale",
        "Moved Workspace ·",
        "worktree is registered at the wrong path",
        "Symlinked Workspace ·",
        "workspace path is not a directory",
        "Archived Misplaced ·",
        "archived Change still has a workspace or worktree registration",
        "interrupted archive",
        "publication branch 'wrong-branch' does not match",
        "published commit 'deadbeef' is not available locally",
        "unsafe permissions",
        ".metadata.lock is not a regular file",
        ".mutation.lock is not a regular file",
    ] {
        assert!(report.contains(expected), "missing {expected:?}:\n{report}");
    }
    assert!(stderr(&output).contains("doctor found"));
    for (path, before) in records_before {
        assert_eq!(fs::read(path).unwrap(), before);
    }
    assert_eq!(
        repo.git(["worktree", "list", "--porcelain"]),
        worktrees_before
    );
    assert_eq!(repo.git(["show-ref"]), refs_before);
}

#[test]
fn doctor_does_not_fetch_promised_objects() {
    let repo = TestRepo::new();
    let change = repo.create_change(None);
    let capsule = change.path.parent().unwrap();
    let missing_oid = "1111111111111111111111111111111111111111";
    let mut record = repo.change_record(capsule);
    record["base_oid"] = missing_oid.into();
    fs::write(
        capsule.join("change.json"),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
    repo.git(["remote", "add", "origin", "ssh://example.test/repository"]);
    repo.git(["config", "remote.origin.promisor", "true"]);
    repo.git(["config", "remote.origin.partialclonefilter", "blob:none"]);
    let ssh = repo.home().join("bin/ssh");
    fs::write(
        &ssh,
        "#!/bin/sh\nprintf 'transport invoked\\n' >> \"$GROVE_TEST_SHIPPING_LOG\"\nexit 1\n",
    )
    .unwrap();
    fs::set_permissions(&ssh, fs::Permissions::from_mode(0o755)).unwrap();

    let output = repo
        .grove()
        .arg("doctor")
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(stdout(&output).contains("creation base '1111111111111111111111111111111111111111'"));
    assert!(
        !repo.shipping_log().contains("transport invoked"),
        "doctor must not invoke Git transport"
    );
}

#[test]
fn doctor_does_not_follow_a_symlinked_repository_directory() {
    let repo = TestRepo::new();
    let change = repo.create_change(None);
    let repository = change.path.parent().unwrap().parent().unwrap();
    let stored = repository.with_extension("stored");
    fs::rename(repository, &stored).unwrap();
    symlink(&stored, repository).unwrap();
    let record = stored.join(&change.id).join("change.json");
    let before = fs::read(&record).unwrap();

    let output = repo
        .grove()
        .arg("doctor")
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(stdout(&output).contains("Grove repository directory is not a directory"));
    assert_eq!(fs::read(record).unwrap(), before);
}
