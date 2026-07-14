mod support;

use support::TestRepo;

#[test]
fn switch_create_creates_branch_worktree_and_navigation_directive() {
    let repo = TestRepo::new();
    let starting_commit = repo.git(["rev-parse", "main"]);
    let common_dir = repo
        .path()
        .join(repo.git(["rev-parse", "--git-common-dir"]))
        .canonicalize()
        .expect("canonical Git common directory");
    let digest = blake3::hash(common_dir.as_os_str().as_encoded_bytes()).to_hex();
    let worktree = repo
        .home()
        .join(".grove")
        .join(format!("repo-{}", &digest[..12]))
        .join("topic");

    repo.grove()
        .args(["switch", "--create", "topic"])
        .assert()
        .success();

    assert_eq!(repo.git(["rev-parse", "topic"]), starting_commit);
    assert_eq!(
        repo.git_from(&worktree, ["branch", "--show-current"]),
        "topic"
    );
    assert_eq!(
        repo.git_from(&worktree, ["rev-parse", "--show-toplevel"]),
        worktree
            .canonicalize()
            .expect("canonical worktree path")
            .display()
            .to_string()
    );
    assert_eq!(repo.navigation(), worktree);
}

#[test]
fn same_named_repositories_get_distinct_worktree_directories() {
    let repo = TestRepo::new();
    let other = repo.create_repo("other/repo");

    repo.grove()
        .args(["switch", "--create", "topic"])
        .assert()
        .success();
    let first_worktree = repo.navigation();

    repo.grove_from(&other)
        .args(["switch", "--create", "topic"])
        .assert()
        .success();
    let second_worktree = repo.navigation();

    assert_ne!(first_worktree, second_worktree);
    assert_eq!(
        repo.git_from(&first_worktree, ["branch", "--show-current"]),
        "topic"
    );
    assert_eq!(
        repo.git_from(&second_worktree, ["branch", "--show-current"]),
        "topic"
    );

    let legacy_worktree = repo.home().join(".grove/repo/legacy");
    repo.git(["branch", "legacy"]);
    repo.git([
        "worktree",
        "add",
        legacy_worktree.to_str().expect("UTF-8 legacy path"),
        "legacy",
    ]);
    repo.grove().args(["switch", "legacy"]).assert().success();
    assert_eq!(
        repo.navigation(),
        legacy_worktree
            .canonicalize()
            .expect("canonical legacy worktree path")
    );
    repo.grove().args(["remove", "legacy"]).assert().success();
    assert!(!legacy_worktree.exists());
}

#[test]
fn switch_create_resolves_and_records_explicit_bases() {
    let repo = TestRepo::new();
    repo.commit_file(repo.path(), "second.txt", "second\n");
    let head = repo.git(["rev-parse", "main"]);
    let parent = repo.git(["rev-parse", "main^"]);
    repo.git(["update-ref", "refs/remotes/origin/main", &parent]);
    repo.git([
        "symbolic-ref",
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/main",
    ]);
    repo.git(["tag", "release", &parent]);

    repo.grove()
        .args(["switch", "--create", "legacy-default"])
        .assert()
        .success();
    assert_eq!(repo.git(["rev-parse", "legacy-default"]), head);
    assert_eq!(repo.config("branch.legacy-default.grove-base-ref"), None);

    let cases = [
        ("from-local", "main", head.as_str(), Some("main")),
        (
            "from-qualified-local",
            "refs/heads/main",
            head.as_str(),
            Some("main"),
        ),
        ("from-remote", "origin/main", parent.as_str(), None),
        ("from-tag", "release", parent.as_str(), None),
        ("from-expression", "main^", parent.as_str(), None),
        ("from-attached", "@", head.as_str(), Some("main")),
    ];
    for (branch, source, expected_oid, expected_parent) in cases {
        repo.grove()
            .args(["switch", "--create", "--from", source, branch])
            .assert()
            .success();
        assert_eq!(repo.git(["rev-parse", branch]), expected_oid);
        assert_eq!(
            repo.git([
                "config",
                "--local",
                "--get",
                &format!("branch.{branch}.grove-base-ref")
            ]),
            source
        );
        assert_eq!(
            repo.git([
                "config",
                "--local",
                "--get",
                &format!("branch.{branch}.grove-base-oid")
            ]),
            expected_oid
        );
        assert_eq!(
            repo.config(&format!("branch.{branch}.grove-parent")),
            expected_parent.map(str::to_owned)
        );
    }

    let attached = repo.navigation();
    repo.commit_file(&attached, "linked-only.txt", "linked only\n");
    repo.grove_from(&attached)
        .args([
            "switch",
            "--create",
            "--from",
            "HEAD^",
            "from-linked-expression",
        ])
        .assert()
        .success();
    assert_eq!(repo.git(["rev-parse", "from-linked-expression"]), head);

    let detached = repo
        .path()
        .parent()
        .expect("repository parent")
        .join("detached");
    repo.git([
        "worktree",
        "add",
        "--detach",
        detached.to_str().expect("UTF-8 path"),
        &parent,
    ]);
    repo.grove_from(&detached)
        .args(["switch", "--create", "--from", "@", "from-detached"])
        .assert()
        .success();
    assert_eq!(repo.git(["rev-parse", "from-detached"]), parent);
    assert_eq!(
        repo.git([
            "config",
            "--local",
            "--get",
            "branch.from-detached.grove-base-ref"
        ]),
        "@"
    );
    assert_eq!(repo.config("branch.from-detached.grove-parent"), None);
}

#[test]
fn switch_from_validation_leaves_repository_untouched() {
    let repo = TestRepo::new();

    repo.grove()
        .args(["switch", "--from", "main", "missing"])
        .assert()
        .failure();
    repo.grove()
        .args(["switch", "--create", "--from", "does-not-exist", "bad-ref"])
        .assert()
        .failure();
    repo.grove()
        .args([
            "switch",
            "--create",
            "--from",
            "HEAD:README.md",
            "bad-object",
        ])
        .assert()
        .failure();

    for branch in ["missing", "bad-ref", "bad-object"] {
        assert!(!repo.branch_exists(branch));
        assert!(!repo.has_lineage(branch));
    }
}

#[test]
fn list_reports_each_worktrees_own_base_without_hiding_invalid_lineage() {
    let repo = TestRepo::new();
    let initial = repo.git(["rev-parse", "main"]);

    repo.git(["branch", "parent"]);
    repo.grove()
        .args(["switch", "--create", "--from", "parent", "dependent"])
        .assert()
        .success();
    let dependent = repo.navigation();
    repo.commit_file(&dependent, "dependent.txt", "dependent\n");

    repo.grove()
        .args(["switch", "--create", "legacy"])
        .assert()
        .success();

    repo.git(["tag", "release"]);
    repo.grove()
        .args(["switch", "--create", "--from", "release", "independent"])
        .assert()
        .success();
    let independent = repo.navigation();
    repo.git([
        "worktree",
        "remove",
        independent.to_str().expect("UTF-8 path"),
    ]);
    repo.grove()
        .args(["switch", "independent"])
        .assert()
        .success();

    repo.git(["branch", "rewritten-parent"]);
    let parent_worktree = repo
        .path()
        .parent()
        .expect("repository parent")
        .join("rewritten-parent");
    repo.git([
        "worktree",
        "add",
        parent_worktree.to_str().expect("UTF-8 path"),
        "rewritten-parent",
    ]);
    let parent_creation_oid = repo.commit_file(&parent_worktree, "parent.txt", "parent\n");
    repo.git([
        "worktree",
        "remove",
        parent_worktree.to_str().expect("UTF-8 path"),
    ]);
    repo.grove()
        .args([
            "switch",
            "--create",
            "--from",
            "rewritten-parent",
            "stale-dependent",
        ])
        .assert()
        .success();
    repo.git(["branch", "-f", "rewritten-parent", &initial]);

    repo.git(["branch", "invalid"]);
    repo.grove().args(["switch", "invalid"]).assert().success();
    repo.git([
        "config",
        "--local",
        "branch.invalid.grove-base-ref",
        "parent",
    ]);

    let output = repo
        .grove()
        .arg("list")
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).expect("Grove stdout is UTF-8");
    let lines = stdout.lines().collect::<Vec<_>>();

    assert_eq!(
        lines
            .first()
            .map(|line| line.split_whitespace().collect::<Vec<_>>()),
        Some(vec!["Branch", "Base", "Changes", "Base↕", "Path"])
    );
    let dependent_row = row_for_branch(&lines, "dependent");
    assert!(dependent_row.contains("parent"), "{dependent_row}");
    assert!(dependent_row.contains("↑1"), "{dependent_row}");

    let independent_row = row_for_branch(&lines, "independent");
    assert!(independent_row.contains("release"), "{independent_row}");

    let stale_row = row_for_branch(&lines, "stale-dependent");
    assert!(
        stale_row.contains(&parent_creation_oid[..12]),
        "{stale_row}"
    );
    assert!(!stale_row.contains("rewritten-parent"), "{stale_row}");

    let legacy_row = row_for_branch(&lines, "legacy");
    assert!(legacy_row.contains("main"), "{legacy_row}");

    let invalid_row = row_for_branch(&lines, "invalid");
    assert!(invalid_row.contains("invalid metadata"), "{invalid_row}");

    repo.grove().args(["remove", "invalid"]).assert().failure();
    repo.grove()
        .args(["remove", "--force", "invalid"])
        .assert()
        .success();

    repo.grove()
        .args(["remove", "independent"])
        .assert()
        .success();
    assert!(!repo.has_lineage("independent"));

    repo.grove()
        .args(["remove", "stale-dependent"])
        .assert()
        .failure();
}

#[test]
fn remove_accepts_integrated_history_shapes_and_rejects_real_unmerged_work() {
    let repo = TestRepo::new();
    let mut tips = std::collections::HashMap::new();
    for branch in ["ancestor", "rebased", "squashed", "unmerged"] {
        repo.grove()
            .args(["switch", "--create", branch])
            .assert()
            .success();
        let worktree = repo.navigation();
        repo.commit_file(&worktree, &format!("{branch}.txt"), &format!("{branch}\n"));
        if branch == "squashed" {
            repo.commit_file(&worktree, "squashed-second.txt", "second\n");
        }
        tips.insert(branch, repo.git(["rev-parse", branch]));
    }

    repo.git(["merge", "--no-ff", "-m", "Merge ancestor", "ancestor"]);
    repo.git(["cherry-pick", tips["rebased"].as_str()]);
    repo.git(["merge", "--squash", "squashed"]);
    repo.git(["commit", "-m", "Squash squashed"]);

    for branch in ["ancestor", "rebased", "squashed"] {
        repo.grove().args(["remove", branch]).assert().success();
        assert!(!repo.branch_exists(branch));
    }

    repo.grove().args(["remove", "unmerged"]).assert().failure();
    assert!(repo.branch_exists("unmerged"));
}

#[test]
fn remove_rejects_unique_content_hidden_in_a_merge_commit() {
    let repo = TestRepo::new();
    repo.grove()
        .args(["switch", "--create", "topic"])
        .assert()
        .success();
    let worktree = repo.navigation();

    let topic_change = repo.commit_file(&worktree, "shared.txt", "shared\n");

    repo.commit_file(repo.path(), "main.txt", "main\n");

    repo.git_from(&worktree, ["merge", "--no-ff", "--no-commit", "main"]);
    std::fs::write(worktree.join("only-in-merge.txt"), "unique resolution\n")
        .expect("write merge-only change");
    repo.git_from(&worktree, ["add", "only-in-merge.txt"]);
    repo.git_from(
        &worktree,
        ["commit", "-m", "Merge main with unique resolution"],
    );
    repo.git(["cherry-pick", &topic_change]);

    let cherry = repo.git(["cherry", "main", "topic"]);
    assert!(
        !cherry.is_empty() && cherry.lines().all(|line| line.starts_with('-')),
        "fixture must reproduce Git cherry omitting the unique merge commit: {cherry}"
    );

    repo.grove().args(["remove", "topic"]).assert().failure();
    assert!(worktree.exists(), "unsafe removal must retain the worktree");
    assert!(
        repo.branch_exists("topic"),
        "unsafe removal must retain the branch"
    );
}

#[test]
fn remove_current_annotated_worktree_clears_lineage_from_primary() {
    let repo = TestRepo::new();
    repo.grove()
        .args(["switch", "--create", "--from", "main", "topic"])
        .assert()
        .success();
    let worktree = repo.navigation();

    repo.grove_from(&worktree).arg("remove").assert().success();

    assert_eq!(
        repo.navigation(),
        repo.path().canonicalize().expect("canonical primary path")
    );
    assert!(!repo.branch_exists("topic"));
    assert!(!repo.has_lineage("topic"));
}

fn row_for_branch<'a>(lines: &'a [&str], branch: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.split_whitespace().nth(1) == Some(branch))
        .unwrap_or_else(|| panic!("missing {branch} row"))
}
