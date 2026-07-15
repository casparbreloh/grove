mod support;

use support::TestRepo;

#[test]
fn new_and_switch_manage_one_agent_lifecycle_per_worktree() {
    let repo = TestRepo::new();
    repo.detach_new("auth");
    let worktree = repo.navigation();
    let first = repo.agent_pids()[0];

    assert_eq!(
        repo.git_from(&worktree, ["branch", "--show-current"]),
        "auth"
    );
    assert!(
        repo.agent_log().contains(&format!(
            "cwd={}",
            worktree
                .canonicalize()
                .expect("canonical worktree")
                .display()
        )),
        "{}",
        repo.agent_log()
    );
    assert_eq!(repo.agent_log().matches("arg=<session>").count(), 1);

    repo.select_project_agent(&worktree, "project");
    repo.detach_switch("auth");
    assert_eq!(repo.agent_pids(), vec![first], "{}", repo.agent_log());
    assert!(!repo.agent_log().contains("arg=<project-session>"));

    repo.terminate_process(first);
    repo.detach_switch("auth");
    let pids = repo.agent_pids();
    assert_eq!(pids.len(), 2, "{}", repo.agent_log());
    assert!(!repo.process_running(first));
    assert!(repo.process_running(pids[1]));
    assert!(repo.agent_log().contains("arg=<project-session>"));
}

#[test]
fn new_validates_agent_configuration_before_mutation() {
    for configure in [
        TestRepo::select_missing_project_agent as fn(&TestRepo),
        TestRepo::use_missing_agent_command,
    ] {
        let repo = TestRepo::new();
        configure(&repo);
        let before = repo.git(["worktree", "list", "--porcelain"]);

        repo.grove().args(["new", "created"]).assert().failure();

        assert!(!repo.branch_exists("created"));
        assert_eq!(repo.git(["worktree", "list", "--porcelain"]), before);
    }
}

#[test]
fn concurrent_switches_reuse_one_session() {
    let repo = TestRepo::new();
    let change = repo.create_change("concurrent agents", None);

    repo.detach_switches_concurrently(&change.branch, 8);

    assert_eq!(repo.agent_pids().len(), 1, "{}", repo.agent_log());
    repo.grove()
        .args(["remove", "--force", &change.branch])
        .assert()
        .success();
}

#[test]
fn switch_requires_an_executable_only_when_starting_the_agent() {
    let missing = TestRepo::new();
    missing.use_missing_agent_command();
    let stderr = missing
        .grove()
        .args(["switch", "main"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    assert!(
        String::from_utf8(stderr)
            .expect("Grove stderr is UTF-8")
            .contains("missing-agent")
    );

    let running = TestRepo::new();
    running.detach_switch("main");
    let pid = running.agent_pids()[0];
    running.remove_configured_agent_executable();
    running.detach_switch("main");
    assert_eq!(running.agent_pids(), vec![pid], "{}", running.agent_log());
    assert!(running.process_running(pid));
}

#[test]
fn switch_defaults_to_pi_without_configuration() {
    let repo = TestRepo::new();
    repo.use_builtin_defaults();

    repo.detach_switch("main");

    let log = repo.agent_log();
    assert!(log.contains("directive=absent"), "{log}");
    assert!(!log.contains("arg=<"), "{log}");
}

#[test]
fn shell_mode_creates_and_switches_without_starting_an_agent() {
    let repo = TestRepo::new();

    repo.grove()
        .args(["new", "--shell", "manual"])
        .assert()
        .success();

    let worktree = repo.navigation();
    assert_eq!(
        repo.git_from(&worktree, ["branch", "--show-current"]),
        "manual"
    );
    assert_eq!(repo.agent_log(), "");
    assert!(!repo.runtime_exists());

    repo.grove()
        .args(["switch", "--shell", "manual"])
        .assert()
        .success();

    assert_eq!(
        repo.navigation().canonicalize().expect("switched worktree"),
        worktree.canonicalize().expect("created worktree")
    );
    assert_eq!(repo.agent_log(), "");
    assert!(!repo.runtime_exists());

    let before = repo.git(["worktree", "list", "--porcelain"]);
    repo.grove().args(["new", "--shell"]).assert().failure();
    assert_eq!(repo.git(["worktree", "list", "--porcelain"]), before);
}

#[test]
fn built_in_agents_name_new_worktrees_from_the_first_prompt() {
    let cases = [
        (
            TestRepo::use_fake_pi as fn(&TestRepo),
            "Fix login redirect",
            "fix-login-redirect",
        ),
        (
            TestRepo::use_fake_claude,
            "Repair token refresh",
            "repair-token-refresh",
        ),
        (
            TestRepo::use_fake_codex,
            "Improve error rendering",
            "improve-error-rendering",
        ),
    ];
    for (configure, prompt, branch) in cases {
        let repo = TestRepo::new();
        configure(&repo);

        let worktree = repo.detach_inferred_new(prompt);

        assert_eq!(
            repo.git_from(&worktree, ["branch", "--show-current"]),
            branch
        );
        assert_eq!(
            repo.git(["branch", "--format=%(refname:short)"])
                .lines()
                .count(),
            2,
            "nameless creation must not create a fallback branch"
        );
    }
}

#[test]
fn new_without_a_branch_rejects_unsupported_agents_before_mutation() {
    let repo = TestRepo::new();

    let stderr = repo
        .grove()
        .arg("new")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(stderr).expect("Grove stderr is UTF-8");

    assert!(stderr.contains("automatic branch naming"), "{stderr}");
    assert_eq!(
        repo.git(["worktree", "list", "--porcelain"])
            .matches("worktree ")
            .count(),
        1,
        "unsupported inference must not create a pending worktree"
    );
}

#[test]
fn detaching_before_the_first_prompt_discards_only_an_untouched_worktree() {
    let clean = TestRepo::new();
    clean.use_fake_pi();
    let output = clean.detach_unnamed_new_without_prompt();
    let terminal = String::from_utf8(output.stdout).expect("Grove terminal is UTF-8");
    assert!(!output.status.success(), "{terminal}");
    assert!(terminal.contains("first prompt"), "{terminal}");
    assert_eq!(
        clean
            .git(["worktree", "list", "--porcelain"])
            .matches("worktree ")
            .count(),
        1,
        "{terminal}"
    );
    assert!(clean.pi_session_files().is_empty());

    let dirty = TestRepo::new();
    dirty.use_fake_pi();
    let (output, worktree) = dirty.detach_dirty_unnamed_new_without_prompt();
    assert!(!output.status.success());
    let output = String::from_utf8_lossy(&output.stdout);
    assert!(output.contains("preserved"), "{output}");
    assert!(worktree.join("agent-created.txt").exists());
    assert!(dirty.process_running(dirty.agent_pids()[0]));
}

#[test]
fn help_exposes_the_minimal_branch_first_command_surface() {
    let repo = TestRepo::new();
    let output = repo
        .grove()
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let help = String::from_utf8(output).expect("Grove help is UTF-8");

    assert!(help.contains("switch"), "{help}");
    assert!(!help.contains("  agent"), "{help}");
    assert!(
        help.lines()
            .any(|line| line.trim_start().starts_with("new")),
        "{help}"
    );

    let new_help = repo
        .grove()
        .args(["new", "--help"])
        .output()
        .expect("run Grove new help");
    let new_help = String::from_utf8(new_help.stdout).expect("Grove new help is UTF-8");
    assert!(
        new_help.contains("Usage: grove new [OPTIONS] [BRANCH]"),
        "{new_help}"
    );
    assert!(new_help.contains("--from <REF>"), "{new_help}");

    let switch_help = repo
        .grove()
        .args(["switch", "--help"])
        .output()
        .expect("run Grove switch help");
    let switch_help = String::from_utf8(switch_help.stdout).expect("Grove switch help is UTF-8");
    assert!(
        switch_help.contains("Usage: grove switch [OPTIONS] [BRANCH]"),
        "{switch_help}"
    );

    let remove_help = repo
        .grove()
        .args(["remove", "--help"])
        .output()
        .expect("run Grove remove help");
    let remove_help = String::from_utf8(remove_help.stdout).expect("Grove remove help is UTF-8");
    assert!(remove_help.contains("Branch to remove"), "{remove_help}");
}

#[test]
fn switch_missing_branch_suggests_new() {
    let repo = TestRepo::new();
    let output = repo
        .grove()
        .args(["switch", "missing"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).expect("Grove stderr is UTF-8");

    assert!(
        stderr.contains("create a change with `grove new`"),
        "{stderr}"
    );
}

#[test]
fn worktree_picker_selects_and_cancels_without_leaking_terminal_state() {
    let repo = TestRepo::new();
    repo.git(["branch", "alpha"]);
    repo.git(["branch", "beta"]);
    repo.grove()
        .args(["switch", "--shell", "alpha"])
        .assert()
        .success();
    let selected = repo.navigation();
    repo.grove()
        .args(["switch", "--shell", "beta"])
        .assert()
        .success();
    let agent_log = repo.agent_log();

    let output = repo.switch_in_pty("beta", b"\x1b[B\x1b[B\x1b[A\r");

    assert!(output.status.success(), "{output:?}");
    let terminal = String::from_utf8(output.stdout).expect("Grove terminal is UTF-8");
    assert_eq!(
        repo.navigation(),
        selected.canonicalize().expect("canonical worktree"),
        "{terminal:?}"
    );
    assert_eq!(repo.agent_log(), agent_log);
    assert!(terminal.contains("Branch"), "{terminal:?}");
    assert_eq!(terminal.matches("Branch").count(), 1, "{terminal:?}");
    assert!(terminal.contains("alpha"), "{terminal:?}");
    assert!(terminal.contains("beta"), "{terminal:?}");
    assert_terminal_restored(&terminal);
    let before = repo.navigation();

    for input in [b"\x1b".as_slice(), b"\x03".as_slice()] {
        let output = repo.switch_in_pty("beta", input);

        assert!(!output.status.success(), "{output:?}");
        let terminal = String::from_utf8(output.stdout).expect("Grove terminal is UTF-8");
        assert!(terminal.contains("selection cancelled"), "{terminal:?}");
        assert_eq!(repo.navigation(), before);
        assert_terminal_restored(&terminal);
    }
}

#[test]
fn worktree_picker_requires_a_terminal() {
    let repo = TestRepo::new();
    repo.create_change("pick me", None);

    let output = repo
        .grove()
        .arg("switch")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).expect("Grove stderr is UTF-8");

    assert!(
        stderr.contains("interactive worktree selection requires a terminal"),
        "{stderr}"
    );
}

#[test]
fn same_named_repositories_get_distinct_worktree_directories() {
    let repo = TestRepo::new();
    let other = repo.create_repo("other/repo");

    let first = repo.create_change("topic", None);
    let second = repo.create_change_from(&other, "topic", None);

    assert_ne!(first.path, second.path);
    assert_eq!(
        repo.git_from(&first.path, ["branch", "--show-current"]),
        first.branch
    );
    assert_eq!(
        repo.git_from(&second.path, ["branch", "--show-current"]),
        second.branch
    );

    let linked_worktree = repo.home().join(".grove/repo/linked");
    repo.git(["branch", "linked"]);
    repo.git([
        "worktree",
        "add",
        linked_worktree.to_str().expect("UTF-8 linked path"),
        "linked",
    ]);
    repo.grove()
        .args(["switch", "--shell", "linked"])
        .assert()
        .success();
    assert_eq!(
        repo.navigation(),
        linked_worktree
            .canonicalize()
            .expect("canonical linked worktree path")
    );
    repo.grove().args(["remove", "linked"]).assert().success();
    assert!(!linked_worktree.exists());
}

#[test]
fn new_resolves_and_records_explicit_bases() {
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

    let default = repo.create_change("default", None);
    assert_eq!(repo.git(["rev-parse", &default.branch]), head);
    assert_eq!(
        repo.config(&format!("branch.{}.grove-base-ref", default.branch)),
        None
    );

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
    let mut attached = None;
    for (task, source, expected_oid, expected_parent) in cases {
        let change = repo.create_change(task, Some(source));
        assert_eq!(repo.git(["rev-parse", &change.branch]), expected_oid);
        assert_eq!(
            repo.git([
                "config",
                "--local",
                "--get",
                &format!("branch.{}.grove-base-ref", change.branch)
            ]),
            source
        );
        assert_eq!(
            repo.git([
                "config",
                "--local",
                "--get",
                &format!("branch.{}.grove-base-oid", change.branch)
            ]),
            expected_oid
        );
        assert_eq!(
            repo.config(&format!("branch.{}.grove-parent", change.branch)),
            expected_parent.map(str::to_owned)
        );
        if task == "from-attached" {
            attached = Some(change.path);
        }
    }

    let attached = attached.expect("attached change");
    repo.commit_file(&attached, "linked-only.txt", "linked only\n");
    let linked = repo.create_change_from(&attached, "from-linked-expression", Some("HEAD^"));
    assert_eq!(repo.git(["rev-parse", &linked.branch]), head);

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
    let detached_change = repo.create_change_from(&detached, "from-detached", Some("@"));
    assert_eq!(repo.git(["rev-parse", &detached_change.branch]), parent);
    assert_eq!(
        repo.git([
            "config",
            "--local",
            "--get",
            &format!("branch.{}.grove-base-ref", detached_change.branch)
        ]),
        "@"
    );
    assert_eq!(
        repo.config(&format!("branch.{}.grove-parent", detached_change.branch)),
        None
    );
}

#[test]
fn new_from_validation_leaves_repository_untouched() {
    let repo = TestRepo::new();
    let before = repo.git(["branch", "--format=%(refname:short)"]);

    repo.grove()
        .args(["new", "--from", "does-not-exist", "bad-ref"])
        .assert()
        .failure();
    repo.grove()
        .args(["new", "--from", "HEAD:README.md", "bad-object"])
        .assert()
        .failure();
    assert_eq!(repo.git(["branch", "--format=%(refname:short)"]), before);
}

#[test]
fn failed_lineage_recording_rolls_back_the_worktree_and_branch() {
    let repo = TestRepo::new();
    let branches_before = repo.git(["branch", "--format=%(refname:short)"]);
    let worktrees_before = repo.git(["worktree", "list", "--porcelain"]);
    let config_lock = repo.path().join(".git/config.lock");
    std::fs::write(&config_lock, "locked by test\n").expect("lock repository config");

    let output = repo
        .grove()
        .args(["new", "--shell", "--from", "main", "rollback-metadata"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    std::fs::remove_file(config_lock).expect("unlock repository config");
    let stderr = String::from_utf8(output).expect("Grove stderr is UTF-8");
    assert!(
        stderr.contains("could not record lineage"),
        "fixture must fail after worktree creation: {stderr}"
    );
    assert_eq!(
        repo.git(["branch", "--format=%(refname:short)"]),
        branches_before
    );
    assert_eq!(
        repo.git(["worktree", "list", "--porcelain"]),
        worktrees_before
    );
    assert!(
        repo.git_optional([
            "config",
            "--local",
            "--get-regexp",
            "^branch\\.rollback-metadata",
        ])
        .is_none()
    );
}

#[test]
fn list_reports_each_worktrees_own_base_without_hiding_invalid_lineage() {
    let repo = TestRepo::new();
    let initial = repo.git(["rev-parse", "main"]);

    repo.git(["branch", "parent"]);
    let dependent = repo.create_change("dependent", Some("parent"));
    repo.commit_file(&dependent.path, "dependent.txt", "dependent\n");

    let default = repo.create_change("default", None);

    repo.git(["tag", "release"]);
    let independent = repo.create_change("independent", Some("release"));
    repo.git([
        "worktree",
        "remove",
        independent.path.to_str().expect("UTF-8 path"),
    ]);
    repo.grove()
        .args(["switch", "--shell", &independent.branch])
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
    let stale = repo.create_change("stale-dependent", Some("rewritten-parent"));
    repo.git(["branch", "-f", "rewritten-parent", &initial]);

    repo.git(["branch", "invalid"]);
    repo.grove()
        .args(["switch", "--shell", "invalid"])
        .assert()
        .success();
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
    let dependent_row = row_for_value(&lines, &dependent.branch);
    assert!(dependent_row.contains("parent"), "{dependent_row}");
    assert!(dependent_row.contains("↑1"), "{dependent_row}");

    let independent_row = row_for_value(&lines, &independent.branch);
    assert!(independent_row.contains("release"), "{independent_row}");

    let stale_row = row_for_value(&lines, &stale.branch);
    assert!(
        stale_row.contains(&parent_creation_oid[..12]),
        "{stale_row}"
    );
    assert!(!stale_row.contains("rewritten-parent"), "{stale_row}");

    let default_row = row_for_value(&lines, &default.branch);
    assert!(default_row.contains("main"), "{default_row}");

    let invalid_row = row_for_value(&lines, "invalid");
    assert!(invalid_row.contains("invalid lineage"), "{invalid_row}");

    repo.grove().args(["remove", "invalid"]).assert().failure();
    repo.grove()
        .args(["remove", "--force", "invalid"])
        .assert()
        .success();

    repo.grove()
        .args(["remove", &independent.branch])
        .assert()
        .success();
    assert!(!repo.has_lineage(&independent.branch));

    repo.grove()
        .args(["remove", &stale.branch])
        .assert()
        .failure();
}

#[test]
fn remove_accepts_integrated_history_shapes_and_rejects_real_unmerged_work() {
    let repo = TestRepo::new();
    let mut tips = std::collections::HashMap::new();
    let mut changes = std::collections::HashMap::new();
    for title in ["ancestor", "rebased", "squashed", "unmerged"] {
        let change = repo.create_change(title, None);
        repo.commit_file(&change.path, &format!("{title}.txt"), &format!("{title}\n"));
        if title == "squashed" {
            repo.commit_file(&change.path, "squashed-second.txt", "second\n");
        }
        tips.insert(title, repo.git(["rev-parse", &change.branch]));
        changes.insert(title, change.branch);
    }

    repo.git([
        "merge",
        "--no-ff",
        "-m",
        "Merge ancestor",
        &changes["ancestor"],
    ]);
    repo.git(["cherry-pick", tips["rebased"].as_str()]);
    repo.git(["merge", "--squash", &changes["squashed"]]);
    repo.git(["commit", "-m", "Squash squashed"]);

    for (command, title) in [
        ("remove", "ancestor"),
        ("delete", "rebased"),
        ("remove", "squashed"),
    ] {
        let branch = &changes[title];
        repo.grove().args([command, branch]).assert().success();
        assert!(!repo.branch_exists(branch));
    }

    repo.grove()
        .args(["remove", &changes["unmerged"]])
        .assert()
        .failure();
    assert!(repo.branch_exists(&changes["unmerged"]));
}

#[test]
fn remove_rejects_unique_content_hidden_in_a_merge_commit() {
    let repo = TestRepo::new();
    let change = repo.create_change("topic", None);
    let worktree = change.path;

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

    let cherry = repo.git(["cherry", "main", &change.branch]);
    assert!(
        !cherry.is_empty() && cherry.lines().all(|line| line.starts_with('-')),
        "fixture must reproduce Git cherry omitting the unique merge commit: {cherry}"
    );

    repo.grove()
        .args(["remove", &change.branch])
        .assert()
        .failure();
    assert!(worktree.exists(), "unsafe removal must retain the worktree");
    assert!(
        repo.branch_exists(&change.branch),
        "unsafe removal must retain the branch"
    );
}

#[test]
fn remove_current_annotated_worktree_clears_lineage_from_primary() {
    let repo = TestRepo::new();
    let change = repo.create_change("topic", Some("main"));

    repo.grove_from(&change.path)
        .arg("remove")
        .assert()
        .success();

    assert_eq!(
        repo.navigation(),
        repo.path().canonicalize().expect("canonical primary path")
    );
    assert!(!repo.branch_exists(&change.branch));
    assert!(!repo.has_lineage(&change.branch));
}

#[test]
fn agent_sessions_are_isolated_by_worktree_during_removal() {
    let repo = TestRepo::new();
    let first = repo.create_change("first agent", None);
    let second = repo.create_change("second agent", None);
    repo.detach_switch(&first.branch);
    repo.detach_switch(&second.branch);
    let pids = repo.agent_pids();
    assert_eq!(pids.len(), 2, "worktrees must have distinct sessions");
    assert!(pids.iter().all(|pid| repo.process_running(*pid)));

    let output = repo
        .grove()
        .args(["remove", &first.branch])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).expect("Grove stderr is UTF-8");
    assert!(stderr.contains("a live agent session"), "{stderr}");
    assert!(first.path.exists());
    assert!(second.path.exists());

    repo.grove_from(&first.path)
        .args(["remove", "--force"])
        .assert()
        .success();

    assert!(!first.path.exists());
    assert!(!repo.branch_exists(&first.branch));
    assert!(!repo.has_lineage(&first.branch));
    for _ in 0..20 {
        if !repo.process_running(pids[0]) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(!repo.process_running(pids[0]));
    assert!(repo.process_running(pids[1]));

    let output = repo
        .grove()
        .args(["remove", &second.branch])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).expect("Grove stderr is UTF-8");
    assert!(stderr.contains("a live agent session"), "{stderr}");
    assert!(second.path.exists());
    repo.grove()
        .args(["remove", "--force", &second.branch])
        .assert()
        .success();
}

#[test]
fn remove_does_not_start_the_runtime_and_runtime_errors_preserve_git_state() {
    let repo = TestRepo::new();
    let removable = repo.create_change("no runtime", None);

    repo.grove()
        .args(["remove", &removable.branch])
        .assert()
        .success();
    assert!(!repo.runtime_exists());

    let protected = repo.create_change("runtime failure", None);
    let endpoint = repo.home().join("x".repeat(200));
    let output = repo
        .grove()
        .env("GROVE_RUNTIME_SOCKET", endpoint)
        .args(["remove", "--force", &protected.branch])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).expect("Grove stderr is UTF-8");
    assert!(
        stderr.contains("failed to inspect embedded agent runtime"),
        "{stderr}"
    );
    assert!(protected.path.exists());
    assert!(repo.branch_exists(&protected.branch));
}

fn row_for_value<'a>(lines: &'a [&str], value: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.split_whitespace().any(|field| field == value))
        .unwrap_or_else(|| panic!("missing {value} row"))
}

fn assert_terminal_restored(terminal: &str) {
    let flags = terminal.split_whitespace().collect::<Vec<_>>();
    assert!(flags.contains(&"icanon"), "{terminal:?}");
    assert!(flags.contains(&"echo"), "{terminal:?}");
}
