mod support;

use std::{fs, path::Path};

use support::{TestChange, TestRepo, stderr, stdout};

#[test]
fn sync_fetches_exact_upstream_archives_and_rebases_safely() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    let stale_main = repo.git(["rev-parse", "main"]);

    let empty = repo.grove().arg("sync").output().unwrap();
    assert!(empty.status.success(), "{}", stderr(&empty));
    assert_eq!(stdout(&empty), "");
    assert_eq!(
        stderr(&empty),
        "✓ Synced 0 Changes: 0 archived, 0 rebased, 0 skipped\n"
    );

    repo.git_from(&publisher, ["checkout", "-b", "unrelated"]);
    repo.commit_file(&publisher, "unrelated.txt", "initial unrelated work\n");
    repo.git_from(
        &publisher,
        ["push", "--set-upstream", "origin", "unrelated"],
    );
    repo.git(["fetch", "origin", "unrelated:refs/remotes/origin/unrelated"]);
    let stale_unrelated = repo.git(["rev-parse", "refs/remotes/origin/unrelated"]);
    repo.git_from(&publisher, ["checkout", "main"]);

    let integrated = repo.create_change(Some("main"));
    repo.set_change_title(&integrated, "Integrated Change");
    repo.git_from(&integrated.path, ["switch", "-c", "synced-local-change"]);
    repo.commit_file(&integrated.path, "integrated.txt", "integrated remotely\n");
    let integrated_tip = repo.change_head(&integrated);

    let remaining = repo.create_change(Some("main"));
    repo.set_change_title(&remaining, "Remaining Change");
    repo.commit_file(&remaining.path, "change.txt", "local change\n");

    let reapplied = repo.create_change(Some("main"));
    repo.set_change_title(&reapplied, "Reapplied Change");
    let reapplied_tip = repo.commit_file(
        &reapplied.path,
        "reapplied.txt",
        "content that must survive sync\n",
    );

    repo.git(["config", "--global", "rebase.updateRefs", "true"]);
    let protected = repo.create_change(Some("main"));
    repo.set_change_title(&protected, "Protected Refs Change");
    let intermediate = repo.commit_file(&protected.path, "first.txt", "first change\n");
    repo.commit_file(&protected.path, "second.txt", "second change\n");
    repo.git(["branch", "unmanaged-snapshot", &intermediate]);

    repo.commit_file(&publisher, "prelude.txt", "upstream prelude\n");
    repo.git_from(
        &publisher,
        ["fetch", repo.path().to_str().unwrap(), &reapplied_tip],
    );
    repo.git_from(&publisher, ["cherry-pick", &reapplied_tip]);
    assert_ne!(
        repo.git_from(&publisher, ["rev-parse", "HEAD"]),
        reapplied_tip
    );
    repo.git_from(&publisher, ["revert", "--no-edit", "HEAD"]);
    repo.git_from(
        &publisher,
        ["fetch", repo.path().to_str().unwrap(), &integrated_tip],
    );
    repo.git_from(&publisher, ["merge", "--squash", "FETCH_HEAD"]);
    repo.git_from(&publisher, ["commit", "-m", "Integrate Grove change"]);
    repo.commit_file(&publisher, "upstream.txt", "new upstream work\n");
    repo.git_from(&publisher, ["tag", "remote-only-tag"]);
    repo.git_from(&publisher, ["push", "origin", "main", "remote-only-tag"]);
    repo.git_from(&publisher, ["checkout", "unrelated"]);
    repo.commit_file(&publisher, "unrelated.txt", "advanced unrelated work\n");
    repo.git_from(&publisher, ["push", "origin", "unrelated"]);
    let fetched_upstream = repo.git_from(&publisher, ["rev-parse", "main"]);

    let output = repo.grove().arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_sync_report(
        &output,
        &[
            ("Integrated Change", "archived", "integrated"),
            ("Remaining Change", "rebased", "upstream"),
            ("Reapplied Change", "rebased", "upstream"),
            ("Protected Refs Change", "rebased", "upstream"),
        ],
    );
    assert_ne!(fetched_upstream, stale_main);
    assert_eq!(repo.git(["rev-parse", "main"]), fetched_upstream);
    assert_eq!(
        repo.git(["rev-parse", "refs/remotes/origin/main"]),
        fetched_upstream
    );
    assert_eq!(
        repo.git(["rev-parse", "refs/remotes/origin/unrelated"]),
        stale_unrelated
    );
    assert!(
        repo.git_optional(["rev-parse", "refs/tags/remote-only-tag"])
            .is_none()
    );

    let integrated_capsule = integrated.path.parent().unwrap();
    assert!(!integrated.path.exists());
    assert!(!repo.branch_exists("synced-local-change"));
    let record = repo.change_record(integrated_capsule);
    assert_eq!(record["state"], "archived");
    assert_eq!(record["outcome"], "integrated");
    assert!(record["archived_at"].is_number());
    assert!(record.get("closing").is_none());

    for change in [&remaining, &reapplied, &protected] {
        assert_eq!(
            repo.git_from(
                &change.path,
                ["merge-base", "--is-ancestor", &fetched_upstream, "HEAD"]
            ),
            ""
        );
        assert_eq!(
            repo.change_record(change.path.parent().unwrap())["state"],
            "active"
        );
    }
    assert_eq!(
        fs::read_to_string(reapplied.path.join("reapplied.txt")).unwrap(),
        "content that must survive sync\n"
    );
    assert_eq!(
        repo.git(["rev-parse", "refs/heads/unmanaged-snapshot"]),
        intermediate
    );
}

#[test]
fn sync_conservatively_preserves_unsafe_topology_and_lineage() {
    let repo = TestRepo::new();
    repo.commit_file(repo.path(), "base.txt", "creation base work\n");
    let publisher = repo.create_local_origin();
    repo.git(["branch", "release"]);

    let other_parent = repo.create_change(Some("release"));
    repo.set_change_title(&other_parent, "Release Parent Change");
    let other_tip = repo.commit_file(&other_parent.path, "release.txt", "release change\n");

    let rewritten = repo.create_change(Some("main"));
    repo.set_change_title(&rewritten, "Rewritten Lineage Change");
    let rewritten_tip = repo.git(["rev-parse", "main^"]);
    repo.git_from(&rewritten.path, ["reset", "--hard", &rewritten_tip]);

    repo.git(["checkout", "-b", "merge-side", "main"]);
    repo.commit_file(repo.path(), "side.txt", "side work\n");
    repo.git(["checkout", "main"]);
    let merged = repo.create_change(Some("main"));
    repo.set_change_title(&merged, "Merge History Change");
    repo.commit_file(&merged.path, "change.txt", "change work\n");
    repo.git_from(
        &merged.path,
        ["merge", "--no-ff", "merge-side", "-m", "Merge side"],
    );
    let merged_tip = repo.change_head(&merged);

    repo.commit_file(&publisher, "upstream.txt", "upstream work\n");
    repo.git_from(&publisher, ["push", "origin", "main"]);
    let output = repo.grove().arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_sync_report(
        &output,
        &[
            ("Release Parent Change", "skipped", "parent"),
            ("Rewritten Lineage Change", "skipped", "creation base"),
            ("Merge History Change", "skipped", "merge history"),
        ],
    );
    for (change, tip) in [
        (&other_parent, other_tip),
        (&rewritten, rewritten_tip),
        (&merged, merged_tip),
    ] {
        assert_eq!(repo.change_head(change), tip);
        assert_eq!(
            repo.change_record(change.path.parent().unwrap())["state"],
            "active"
        );
    }

    let diverged = TestRepo::new();
    let publisher = diverged.create_local_origin();
    diverged.commit_file(diverged.path(), "base.txt", "recorded base\n");
    diverged.git(["push", "origin", "main"]);
    diverged.git_from(&publisher, ["pull", "--ff-only"]);
    let base = diverged.git(["rev-parse", "main"]);
    let change = diverged.create_change(Some("main"));
    diverged.set_change_title(&change, "Diverged Upstream Change");
    let tip = diverged.commit_file(&change.path, "change.txt", "local change\n");
    diverged.git_from(&publisher, ["reset", "--hard", &format!("{base}^")]);
    diverged.commit_file(&publisher, "replacement.txt", "replacement history\n");
    diverged.git_from(&publisher, ["push", "--force", "origin", "HEAD:main"]);
    let fetched_upstream = diverged.git_from(&publisher, ["rev-parse", "main"]);
    let output = diverged.grove().arg("sync").output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("cannot be fast-forwarded"),
        "{output:?}"
    );
    assert_eq!(diverged.change_head(&change), tip);
    assert_eq!(diverged.git(["rev-parse", "main"]), base);
    assert_eq!(
        diverged.git(["rev-parse", "refs/remotes/origin/main"]),
        fetched_upstream
    );
}

#[test]
fn sync_validation_and_fetch_failures_happen_before_mutation() {
    {
        let repo = TestRepo::new();
        let publisher = repo.create_local_origin();
        let stale_main = repo.git(["rev-parse", "main"]);
        let stale_upstream = repo.git(["rev-parse", "refs/remotes/origin/main"]);
        let change = repo.create_change(Some("main"));
        let head_before = repo.commit_file(&change.path, "change.txt", "local change\n");
        let record_path = change.path.parent().unwrap().join("change.json");
        fs::write(&record_path, b"{ malformed Change metadata\n").unwrap();
        let record_before = fs::read(&record_path).unwrap();

        repo.commit_file(&publisher, "upstream.txt", "remote advance\n");
        repo.git_from(&publisher, ["push", "origin", "main"]);
        let output = repo.grove().arg("sync").output().unwrap();

        assert!(!output.status.success(), "{output:?}");
        assert!(
            stderr(&output).contains("invalid change record"),
            "{output:?}"
        );
        assert_eq!(repo.git(["rev-parse", "main"]), stale_main);
        assert_eq!(
            repo.git(["rev-parse", "refs/remotes/origin/main"]),
            stale_upstream
        );
        assert_eq!(repo.change_head(&change), head_before);
        assert_eq!(fs::read(record_path).unwrap(), record_before);
    }

    {
        let repo = TestRepo::new();
        repo.create_local_origin();
        let change = repo.create_change(Some("main"));
        let content_path = change.path.join("change.txt");
        repo.commit_file(&change.path, "change.txt", "committed change\n");

        let capsule = change.path.parent().unwrap();
        let head_before = repo.change_head(&change);
        let status_before = repo.git_from(&change.path, ["status", "--porcelain=v1"]);
        let content_before = fs::read(&content_path).unwrap();
        let record_before = fs::read(capsule.join("change.json")).unwrap();
        assert_eq!(repo.change_record(capsule)["state"], "active");

        let origin = repo.git(["remote", "get-url", "origin"]);
        fs::remove_dir_all(origin).unwrap();
        let output = repo.grove().arg("sync").output().unwrap();

        assert!(!output.status.success(), "{output:?}");
        let error = stderr(&output);
        assert!(error.contains("failed to fetch merge ref"), "{error}");
        assert!(change.path.exists());
        assert_eq!(repo.change_head(&change), head_before);
        assert_eq!(
            repo.git_from(&change.path, ["status", "--porcelain=v1"]),
            status_before
        );
        assert_eq!(fs::read(content_path).unwrap(), content_before);
        assert_eq!(
            fs::read(capsule.join("change.json")).unwrap(),
            record_before
        );
    }

    {
        let repo = TestRepo::new();
        let publisher = repo.create_local_origin();
        let stale_upstream = repo.git(["rev-parse", "refs/remotes/origin/main"]);

        let change = repo.create_change(Some("main"));
        repo.commit_file(&change.path, "change.txt", "integrated remotely\n");
        let change_tip = repo.change_head(&change);
        repo.git_from(
            &publisher,
            ["fetch", repo.path().to_str().unwrap(), &change_tip],
        );
        repo.git_from(
            &publisher,
            [
                "merge",
                "--no-ff",
                "-m",
                "Integrate current Change",
                "FETCH_HEAD",
            ],
        );
        repo.commit_file(&publisher, "upstream.txt", "remote advance\n");
        repo.git_from(&publisher, ["push", "origin", "main"]);
        assert_ne!(
            repo.git_from(&publisher, ["rev-parse", "main"]),
            stale_upstream
        );

        let head_before = repo.change_head(&change);
        let record_path = change.path.parent().unwrap().join("change.json");
        let record_before = fs::read(&record_path).unwrap();
        let worktree_bytes = |path: &Path| {
            let mut files = fs::read_dir(path)
                .unwrap()
                .map(|entry| {
                    let path = entry.unwrap().path();
                    (
                        path.file_name().unwrap().to_owned(),
                        fs::read(path).unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            files.sort_by(|left, right| left.0.cmp(&right.0));
            files
        };
        let worktree_before = worktree_bytes(&change.path);

        let output = repo.grove_from(&change.path).arg("sync").output().unwrap();

        assert!(!output.status.success(), "{output:?}");
        assert!(stderr(&output).contains("primary worktree"), "{output:?}");
        assert_eq!(
            repo.git(["rev-parse", "refs/remotes/origin/main"]),
            stale_upstream
        );
        assert!(change.path.exists());
        assert_eq!(repo.change_head(&change), head_before);
        assert_eq!(worktree_bytes(&change.path), worktree_before);
        assert_eq!(fs::read(record_path).unwrap(), record_before);
    }

    {
        let repo = TestRepo::new();
        let publisher = repo.create_local_origin();
        let stale_main = repo.git(["rev-parse", "main"]);
        let stale_upstream = repo.git(["rev-parse", "refs/remotes/origin/main"]);
        fs::write(
            repo.path().join("README.md"),
            "# Local uncommitted change\n",
        )
        .unwrap();
        repo.commit_file(&publisher, "README.md", "# Conflicting upstream change\n");
        repo.git_from(&publisher, ["push", "origin", "main"]);
        repo.git(["config", "merge.autostash", "true"]);

        let output = repo.grove().arg("sync").output().unwrap();

        assert!(!output.status.success(), "{output:?}");
        assert!(
            stderr(&output).contains("primary worktree has uncommitted changes"),
            "{output:?}"
        );
        assert_eq!(repo.git(["rev-parse", "main"]), stale_main);
        assert_eq!(
            repo.git(["rev-parse", "refs/remotes/origin/main"]),
            stale_upstream
        );
        assert_eq!(
            fs::read_to_string(repo.path().join("README.md")).unwrap(),
            "# Local uncommitted change\n"
        );
    }
}

#[test]
fn sync_aborts_conflicts_continues_rebases_and_skips_dirty_changes() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    let stale_main = repo.git(["rev-parse", "main"]);

    let conflicting = repo.create_change(Some("main"));
    repo.set_change_title(&conflicting, "Preserve Conflicting Change");
    let conflicting_tip =
        repo.commit_file(&conflicting.path, "README.md", "# Conflicting change\n");

    let rebased = repo.create_change(Some("main"));
    repo.set_change_title(&rebased, "Continue Clean Rebase");
    let rebased_tip = repo.commit_file(&rebased.path, "clean.txt", "clean change\n");

    let dirty = repo.create_change(Some("main"));
    repo.set_change_title(&dirty, "Skip Dirty Change");
    let dirty_tip = repo.commit_file(&dirty.path, "dirty.txt", "committed state\n");
    fs::write(dirty.path.join("dirty.txt"), "uncommitted state\n").unwrap();
    let dirty_status = repo.git_from(&dirty.path, ["status", "--porcelain=v1"]);

    repo.commit_file(&publisher, "README.md", "# Upstream change\n");
    repo.git_from(&publisher, ["push", "origin", "main"]);
    let fetched_upstream = repo.git_from(&publisher, ["rev-parse", "main"]);

    let output = repo
        .grove()
        .arg("sync")
        .assert()
        .success()
        .get_output()
        .clone();
    assert_sync_report(
        &output,
        &[
            ("Preserve Conflicting Change", "skipped", "rebase failed"),
            ("Continue Clean Rebase", "rebased", "upstream"),
            ("Skip Dirty Change", "skipped", "uncommitted"),
        ],
    );

    assert_ne!(fetched_upstream, stale_main);
    assert_eq!(repo.git(["rev-parse", "main"]), fetched_upstream);
    assert_eq!(
        repo.git(["rev-parse", "refs/remotes/origin/main"]),
        fetched_upstream
    );

    assert_eq!(
        repo.git_from(&conflicting.path, ["rev-parse", "HEAD"]),
        conflicting_tip
    );
    assert_eq!(
        repo.git_from(&conflicting.path, ["status", "--porcelain=v1"]),
        ""
    );
    for name in ["rebase-merge", "rebase-apply"] {
        let metadata = repo.git_from(
            &conflicting.path,
            ["rev-parse", "--path-format=absolute", "--git-path", name],
        );
        assert!(!Path::new(&metadata).exists(), "{metadata} still exists");
    }

    assert_ne!(
        repo.git_from(&rebased.path, ["rev-parse", "HEAD"]),
        rebased_tip
    );
    assert_eq!(
        repo.git_from(&rebased.path, ["rev-parse", "HEAD^"]),
        fetched_upstream
    );
    assert_eq!(
        repo.git_from(&rebased.path, ["status", "--porcelain=v1"]),
        ""
    );

    assert_eq!(repo.git_from(&dirty.path, ["rev-parse", "HEAD"]), dirty_tip);
    assert_eq!(
        repo.git_from(&dirty.path, ["status", "--porcelain=v1"]),
        dirty_status
    );
    assert_eq!(
        fs::read_to_string(dirty.path.join("dirty.txt")).unwrap(),
        "uncommitted state\n"
    );
}

#[test]
fn sync_skips_busy_locked_and_missing_changes_while_rebasing_an_eligible_change() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();

    let (agent, agent_gate) = repo.start_blocking_new();
    let busy_capsule = repo.change_capsules().pop().expect("busy Change capsule");
    let busy = TestChange {
        id: busy_capsule
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        path: busy_capsule.join("workspace"),
    };
    repo.set_change_title(&busy, "Busy Agent Change");
    let busy_tip = repo.commit_file(&busy.path, "busy.txt", "busy change\n");

    let locked = repo.create_change(Some("main"));
    repo.set_change_title(&locked, "Locked Worktree Change");
    let locked_tip = repo.commit_file(&locked.path, "locked.txt", "locked change\n");

    let missing = repo.create_change(Some("main"));
    repo.set_change_title(&missing, "Missing Worktree Change");
    let missing_tip = repo.commit_file(&missing.path, "missing.txt", "missing change\n");
    let missing_git_dir =
        Path::new(&repo.git_from(&missing.path, ["rev-parse", "--absolute-git-dir"])).to_owned();

    let eligible = repo.create_change(Some("main"));
    repo.set_change_title(&eligible, "Eligible Rebase Change");
    let eligible_tip = repo.commit_file(&eligible.path, "eligible.txt", "eligible change\n");

    repo.git([
        "worktree",
        "lock",
        "--reason",
        "Grove sync test",
        locked.path.to_str().unwrap(),
    ]);
    fs::remove_dir_all(&missing.path).unwrap();
    let inventory_before = repo.git(["worktree", "list", "--porcelain"]);
    assert!(
        inventory_before.contains(&format!("worktree {}", missing.path.display()))
            && inventory_before.contains("prunable"),
        "{inventory_before}"
    );

    let skipped = [&busy, &locked, &missing];
    let records_before = skipped
        .iter()
        .map(|change| fs::read(change.path.parent().unwrap().join("change.json")).unwrap())
        .collect::<Vec<_>>();

    repo.commit_file(&publisher, "upstream.txt", "remote advance\n");
    repo.git_from(&publisher, ["push", "origin", "main"]);
    let upstream = repo.git_from(&publisher, ["rev-parse", "main"]);

    let output = repo.grove().arg("sync").output().unwrap();
    repo.release_blocking_agent(agent, &agent_gate);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_sync_report(
        &output,
        &[
            ("Busy Agent Change", "skipped", "already open"),
            ("Locked Worktree Change", "skipped", "locked"),
            ("Missing Worktree Change", "skipped", "missing"),
            ("Eligible Rebase Change", "rebased", "upstream"),
        ],
    );

    for ((change, record_before), tip_before) in
        skipped
            .iter()
            .zip(&records_before)
            .zip([busy_tip, locked_tip, missing_tip])
    {
        if change.path.exists() {
            assert_eq!(repo.change_head(change), tip_before);
        } else {
            assert_eq!(change.id, missing.id);
        }
        assert_eq!(
            fs::read(change.path.parent().unwrap().join("change.json")).unwrap(),
            *record_before
        );
        assert_eq!(
            repo.change_record(change.path.parent().unwrap())["state"],
            "active"
        );
    }

    assert!(busy.path.exists());
    assert!(locked.path.exists());
    assert!(!missing.path.exists());
    assert!(missing.path.parent().unwrap().exists());
    assert!(missing_git_dir.exists());
    let inventory_after = repo.git(["worktree", "list", "--porcelain"]);
    assert!(
        inventory_after.contains(&format!("worktree {}", locked.path.display()))
            && inventory_after.contains("locked Grove sync test"),
        "{inventory_after}"
    );
    assert!(
        inventory_after.contains(&format!("worktree {}", missing.path.display()))
            && inventory_after.contains("prunable"),
        "{inventory_after}"
    );

    assert!(eligible.path.exists());
    assert_ne!(repo.change_head(&eligible), eligible_tip);
    assert_eq!(
        repo.git_from(&eligible.path, ["rev-parse", "HEAD^"]),
        upstream
    );
    assert_eq!(
        repo.git_from(&eligible.path, ["show", "HEAD:eligible.txt"]),
        "eligible change"
    );
    assert_eq!(
        repo.git_from(&eligible.path, ["show", "HEAD:upstream.txt"]),
        "remote advance"
    );
}

fn assert_sync_report(output: &std::process::Output, expected: &[(&str, &str, &str)]) {
    assert_eq!(stdout(output), "");
    let report = stderr(output);
    let lines = report.split_terminator('\n').collect::<Vec<_>>();
    assert_eq!(lines.len(), expected.len() + 2, "{report}");

    let rows = &lines[..expected.len()];
    for (title, outcome, reason) in expected {
        let marker = match *outcome {
            "archived" => "- ",
            "rebased" => "↑ ",
            "skipped" => "○ ",
            _ => panic!("unexpected sync outcome {outcome}"),
        };
        let title = title.to_lowercase();
        let outcome = outcome.to_lowercase();
        let reason = reason.to_lowercase();
        let matches = rows
            .iter()
            .filter(|row| {
                let normalized = row.to_lowercase();
                row.starts_with(marker)
                    && normalized.contains(&title)
                    && normalized.contains(&outcome)
                    && normalized.contains(&reason)
            })
            .count();
        assert_eq!(
            matches, 1,
            "expected one sync row with marker {marker:?}, title {title:?}, outcome {outcome:?}, and reason {reason:?}: {report}"
        );
    }
    assert_eq!(lines[expected.len()], "", "{report}");

    let archived = expected
        .iter()
        .filter(|(_, outcome, _)| *outcome == "archived")
        .count();
    let rebased = expected
        .iter()
        .filter(|(_, outcome, _)| *outcome == "rebased")
        .count();
    let skipped = expected
        .iter()
        .filter(|(_, outcome, _)| *outcome == "skipped")
        .count();
    assert_eq!(
        lines[expected.len() + 1],
        format!(
            "✓ Synced {} Changes: {archived} archived, {rebased} rebased, {skipped} skipped",
            expected.len()
        ),
        "{report}"
    );
}
