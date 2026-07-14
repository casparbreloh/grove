mod support;

use support::TestRepo;

#[test]
fn agent_rejects_prompt_templates() {
    let repo = TestRepo::new();
    repo.use_prompt_template();

    let output = repo
        .grove()
        .arg("agent")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).expect("Grove stderr is UTF-8");
    assert!(
        stderr.contains("{prompt} is no longer supported; remove it from the agent command"),
        "{stderr}"
    );
}

#[test]
fn agent_runs_the_configured_command_in_the_worktree() {
    let repo = TestRepo::new();
    let change = repo.create_change("agent runtime", None);

    let terminal = repo.detach_agent(&change.path, None);

    assert_eq!(
        repo.agent_log(),
        format!(
            "cwd={}\ndirective=absent\narg=<session>\narg=<space value>\narg=<quote'\">\narg=<>\n",
            change
                .path
                .canonicalize()
                .expect("canonical worktree")
                .display()
        )
    );
    assert!(!terminal.contains("[grove-"), "{terminal}");
}

#[test]
fn agent_reattaches_and_named_agents_coexist() {
    let repo = TestRepo::new();
    let change = repo.create_change("persistent agents", None);

    repo.detach_agent(&change.path, None);
    repo.detach_agent(&change.path, None);
    repo.detach_agent(&change.path, Some("project"));

    let log = repo.agent_log();
    assert_eq!(log.matches("arg=<session>").count(), 1, "{log}");
    assert_eq!(log.matches("arg=<project-session>").count(), 1, "{log}");
}

#[test]
fn concurrent_agent_launches_reuse_one_session() {
    let repo = TestRepo::new();
    let change = repo.create_change("concurrent agents", None);

    repo.detach_agents_concurrently(&change.path, 8);

    assert_eq!(repo.agent_pids().len(), 1, "{}", repo.agent_log());
    repo.grove()
        .args(["remove", "--force", &change.id])
        .assert()
        .success();
}

#[test]
fn project_agent_selection_works_in_an_ordinary_worktree() {
    let repo = TestRepo::new();
    repo.select_project_agent(repo.path(), "project");

    repo.detach_agent(repo.path(), None);

    let log = repo.agent_log();
    assert!(log.contains("arg=<project-session>"), "{log}");
    assert!(
        log.contains(&format!(
            "cwd={}",
            repo.path()
                .canonicalize()
                .expect("canonical worktree")
                .display()
        )),
        "{log}"
    );
}

#[test]
fn agent_launch_errors_are_reported_before_terminal_attachment() {
    let repo = TestRepo::new();
    repo.use_missing_agent_command();

    let output = repo
        .grove()
        .arg("agent")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).expect("Grove stderr is UTF-8");
    assert!(stderr.contains("missing-agent"), "{stderr}");
}

#[test]
fn agent_defaults_to_pi_without_configuration() {
    let repo = TestRepo::new();
    repo.use_builtin_defaults();

    repo.detach_agent(repo.path(), None);

    let log = repo.agent_log();
    assert!(log.contains("directive=absent"), "{log}");
    assert!(!log.contains("arg=<"), "{log}");
}

#[test]
fn switch_create_without_a_title_creates_an_untitled_change_without_an_agent() {
    let repo = TestRepo::new();

    repo.grove().args(["switch", "-c"]).assert().success();

    let worktree = repo.navigation();
    let branch = repo.git_from(&worktree, ["branch", "--show-current"]);
    assert!(
        branch.starts_with("c-"),
        "unexpected change branch: {branch}"
    );
    assert_eq!(branch.len(), 14);
    assert_eq!(
        repo.config(&format!("branch.{branch}.grove-change")),
        Some("true".to_owned())
    );
    assert_eq!(repo.agent_log(), "");

    let output = repo
        .grove()
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("Grove stdout is UTF-8");
    assert!(output.contains("(untitled)"), "{output}");
    assert!(output.contains(&branch), "{output}");
}

#[test]
fn switch_create_with_a_title_records_it_on_a_stable_change() {
    let repo = TestRepo::new();
    let starting_commit = repo.git(["rev-parse", "main"]);
    let common_dir = repo
        .path()
        .join(repo.git(["rev-parse", "--git-common-dir"]))
        .canonicalize()
        .expect("canonical Git common directory");
    let digest = blake3::hash(common_dir.as_os_str().as_encoded_bytes()).to_hex();
    repo.grove()
        .args(["switch", "--create", "Fix OAuth refresh race"])
        .assert()
        .success();
    let change_path = repo.navigation();
    let change_id = repo.git_from(&change_path, ["branch", "--show-current"]);
    let expected = repo
        .home()
        .join(".grove")
        .join(format!("repo-{}", &digest[..12]))
        .join(&change_id);

    assert_eq!(repo.git(["rev-parse", &change_id]), starting_commit);
    assert_eq!(change_path, expected);
    assert_eq!(
        repo.git_from(&change_path, ["rev-parse", "--show-toplevel"]),
        change_path
            .canonicalize()
            .expect("canonical worktree path")
            .display()
            .to_string()
    );
    assert_eq!(
        repo.config(&format!("branch.{change_id}.description")),
        Some("Fix OAuth refresh race".to_owned())
    );
    assert_eq!(repo.agent_log(), "");

    let long_task = "Investigate why authentication refresh races can silently discard newly issued access tokens";
    repo.grove()
        .args(["switch", "--create", long_task])
        .assert()
        .success();
    let other_id = repo.git_from(&repo.navigation(), ["branch", "--show-current"]);
    assert_ne!(change_id, other_id);
    let output = repo.grove().arg("list").output().expect("run Grove list");
    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).expect("Grove stdout is UTF-8");
    let shortened = long_task.chars().take(59).chain(['…']).collect::<String>();
    assert!(output.contains(&shortened), "{output}");
    assert!(!output.contains(long_task), "{output}");
}

#[test]
fn help_exposes_creation_on_switch_instead_of_new() {
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
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("new")),
        "{help}"
    );
    repo.grove().arg("new").assert().failure();
}

#[test]
fn switch_navigates_without_launching_an_agent() {
    let repo = TestRepo::new();
    let change = repo.create_change("Fix OAuth refresh race", None);
    let agent_log = repo.agent_log();

    repo.grove().args(["switch", &change.id]).assert().success();

    assert_eq!(
        repo.navigation(),
        change.path.canonicalize().expect("canonical worktree")
    );
    assert_eq!(repo.agent_log(), agent_log);
}

#[test]
fn switch_without_an_id_picks_a_worktree() {
    let repo = TestRepo::new();
    let change = repo.create_change("Fix OAuth refresh race", None);
    let agent_log = repo.agent_log();

    let output = repo
        .grove()
        .arg("switch")
        .write_stdin("1\n")
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(
        repo.navigation(),
        change.path.canonicalize().expect("canonical worktree")
    );
    assert_eq!(repo.agent_log(), agent_log);
    let stderr = String::from_utf8(output.stderr).expect("Grove stderr is UTF-8");
    assert!(stderr.contains("Fix OAuth refresh race"), "{stderr}");
    assert!(stderr.contains(&change.id), "{stderr}");
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
        first.id
    );
    assert_eq!(
        repo.git_from(&second.path, ["branch", "--show-current"]),
        second.id
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

    let legacy = repo.create_change("legacy-default", None);
    assert_eq!(repo.git(["rev-parse", &legacy.id]), head);
    assert_eq!(
        repo.config(&format!("branch.{}.grove-base-ref", legacy.id)),
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
        assert_eq!(repo.git(["rev-parse", &change.id]), expected_oid);
        assert_eq!(
            repo.git([
                "config",
                "--local",
                "--get",
                &format!("branch.{}.grove-base-ref", change.id)
            ]),
            source
        );
        assert_eq!(
            repo.git([
                "config",
                "--local",
                "--get",
                &format!("branch.{}.grove-base-oid", change.id)
            ]),
            expected_oid
        );
        assert_eq!(
            repo.config(&format!("branch.{}.grove-parent", change.id)),
            expected_parent.map(str::to_owned)
        );
        if task == "from-attached" {
            attached = Some(change.path);
        }
    }

    let attached = attached.expect("attached change");
    repo.commit_file(&attached, "linked-only.txt", "linked only\n");
    let linked = repo.create_change_from(&attached, "from-linked-expression", Some("HEAD^"));
    assert_eq!(repo.git(["rev-parse", &linked.id]), head);

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
    assert_eq!(repo.git(["rev-parse", &detached_change.id]), parent);
    assert_eq!(
        repo.git([
            "config",
            "--local",
            "--get",
            &format!("branch.{}.grove-base-ref", detached_change.id)
        ]),
        "@"
    );
    assert_eq!(
        repo.config(&format!("branch.{}.grove-parent", detached_change.id)),
        None
    );
}

#[test]
fn switch_from_validation_leaves_repository_untouched() {
    let repo = TestRepo::new();
    let before = repo.git(["branch", "--format=%(refname:short)"]);

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
    assert_eq!(repo.git(["branch", "--format=%(refname:short)"]), before);
}

#[test]
fn list_reports_each_worktrees_own_base_without_hiding_invalid_lineage() {
    let repo = TestRepo::new();
    let initial = repo.git(["rev-parse", "main"]);

    repo.git(["branch", "parent"]);
    let dependent = repo.create_change("dependent", Some("parent"));
    repo.commit_file(&dependent.path, "dependent.txt", "dependent\n");

    let legacy = repo.create_change("legacy", None);

    repo.git(["tag", "release"]);
    let independent = repo.create_change("independent", Some("release"));
    repo.git([
        "worktree",
        "remove",
        independent.path.to_str().expect("UTF-8 path"),
    ]);
    repo.grove()
        .args(["switch", &independent.id])
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
        Some(vec!["Change", "ID", "Base", "Changes", "Base↕", "Path"])
    );
    let dependent_row = row_for_value(&lines, &dependent.id);
    assert!(dependent_row.contains("parent"), "{dependent_row}");
    assert!(dependent_row.contains("↑1"), "{dependent_row}");

    let independent_row = row_for_value(&lines, &independent.id);
    assert!(independent_row.contains("release"), "{independent_row}");

    let stale_row = row_for_value(&lines, &stale.id);
    assert!(
        stale_row.contains(&parent_creation_oid[..12]),
        "{stale_row}"
    );
    assert!(!stale_row.contains("rewritten-parent"), "{stale_row}");

    let legacy_row = row_for_value(&lines, &legacy.id);
    assert!(legacy_row.contains("main"), "{legacy_row}");

    let invalid_row = row_for_value(&lines, "invalid");
    assert!(invalid_row.contains("invalid metadata"), "{invalid_row}");

    repo.grove().args(["remove", "invalid"]).assert().failure();
    repo.grove()
        .args(["remove", "--force", "invalid"])
        .assert()
        .success();

    repo.grove()
        .args(["remove", &independent.id])
        .assert()
        .success();
    assert!(!repo.has_lineage(&independent.id));

    repo.grove().args(["remove", &stale.id]).assert().failure();
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
        tips.insert(title, repo.git(["rev-parse", &change.id]));
        changes.insert(title, change.id);
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

    for title in ["ancestor", "rebased", "squashed"] {
        let id = &changes[title];
        repo.grove().args(["remove", id]).assert().success();
        assert!(!repo.branch_exists(id));
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

    let cherry = repo.git(["cherry", "main", &change.id]);
    assert!(
        !cherry.is_empty() && cherry.lines().all(|line| line.starts_with('-')),
        "fixture must reproduce Git cherry omitting the unique merge commit: {cherry}"
    );

    repo.grove().args(["remove", &change.id]).assert().failure();
    assert!(worktree.exists(), "unsafe removal must retain the worktree");
    assert!(
        repo.branch_exists(&change.id),
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
    assert!(!repo.branch_exists(&change.id));
    assert!(!repo.has_lineage(&change.id));
}

#[test]
fn same_named_agents_are_isolated_by_worktree_during_removal() {
    let repo = TestRepo::new();
    let first = repo.create_change("first agent", None);
    let second = repo.create_change("second agent", None);
    repo.detach_agent(&first.path, None);
    repo.detach_agent(&first.path, Some("project"));
    repo.detach_agent(&second.path, None);
    let pids = repo.agent_pids();
    assert_eq!(
        pids.len(),
        3,
        "same-named agents must have distinct sessions"
    );
    assert!(pids.iter().all(|pid| repo.process_running(*pid)));

    let output = repo
        .grove()
        .args(["remove", &first.id])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).expect("Grove stderr is UTF-8");
    assert!(stderr.contains("2 live agent sessions"), "{stderr}");
    assert!(first.path.exists());
    assert!(second.path.exists());

    repo.grove_from(&first.path)
        .args(["remove", "--force"])
        .assert()
        .success();

    assert!(!first.path.exists());
    assert!(!repo.branch_exists(&first.id));
    assert!(!repo.has_lineage(&first.id));
    for _ in 0..20 {
        if !repo.process_running(pids[0]) && !repo.process_running(pids[1]) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(!repo.process_running(pids[0]));
    assert!(!repo.process_running(pids[1]));
    assert!(repo.process_running(pids[2]));

    let output = repo
        .grove()
        .args(["remove", &second.id])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).expect("Grove stderr is UTF-8");
    assert!(stderr.contains("1 live agent session"), "{stderr}");
    assert!(second.path.exists());
    repo.grove()
        .args(["remove", "--force", &second.id])
        .assert()
        .success();
}

#[test]
fn remove_does_not_start_the_runtime_and_runtime_errors_preserve_git_state() {
    let repo = TestRepo::new();
    let removable = repo.create_change("no runtime", None);

    repo.grove()
        .args(["remove", &removable.id])
        .assert()
        .success();
    assert!(!repo.runtime_exists());

    let protected = repo.create_change("runtime failure", None);
    let endpoint = repo.home().join("x".repeat(200));
    let output = repo
        .grove()
        .env("GROVE_RUNTIME_SOCKET", endpoint)
        .args(["remove", "--force", &protected.id])
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
    assert!(repo.branch_exists(&protected.id));
    assert!(repo.has_lineage(&protected.id));
}

fn row_for_value<'a>(lines: &'a [&str], value: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.split_whitespace().any(|field| field == value))
        .unwrap_or_else(|| panic!("missing {value} row"))
}
