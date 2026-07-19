mod support;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use support::{TestChange, TestRepo};

#[test]
fn command_and_shell_surface_is_small_and_navigation_is_explicit() {
    let repo = TestRepo::new();
    let help = stdout(repo.grove().arg("--help").assert().success().get_output());
    for command in ["new", "switch", "list", "sync", "remove", "init"] {
        assert!(help.contains(command), "{help}");
    }
    assert!(!help.contains("  agent"), "{help}");

    for (command, usage, flag) in [
        ("new", "Usage: grove new [OPTIONS]", "--from <REF>"),
        ("switch", "Usage: grove switch [OPTIONS]", "--shell"),
        ("remove", "Usage: grove remove [OPTIONS]", "--force"),
    ] {
        let output = repo
            .grove()
            .args([command, "--help"])
            .assert()
            .success()
            .get_output()
            .clone();
        let text = stdout(&output);
        assert!(text.contains(usage), "{text}");
        assert!(text.contains(flag), "{text}");
        assert!(!text.contains("BRANCH"), "{text}");
        if command == "new" {
            assert!(
                text.contains("additional, asynchronous provider request"),
                "{text}"
            );
        }
        repo.grove()
            .args([command, "manual-name"])
            .assert()
            .failure();
    }

    for shell in ["fish", "zsh"] {
        let output = repo
            .grove()
            .args(["init", shell])
            .assert()
            .success()
            .get_output()
            .clone();
        let script = stdout(&output);
        assert!(script.contains("GROVE_DIRECTIVE_CD_FILE"), "{script}");
        assert!(script.contains("command grove"), "{script}");
        assert!(script.contains("COMPLETE"), "{script}");
    }

    let missing_wrapper = TestRepo::new();
    let output = missing_wrapper
        .grove()
        .env_remove("GROVE_DIRECTIVE_CD_FILE")
        .args(["new", "--shell"])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(
        stderr(&output).contains("shell integration is not loaded"),
        "{}",
        stderr(&output)
    );
    assert!(missing_wrapper.change_capsules().is_empty());
    assert_eq!(
        missing_wrapper.git(["branch", "--format=%(refname:short)"]),
        "main"
    );

    let invalid_target = TestRepo::new();
    let output = invalid_target
        .grove()
        .env("GROVE_DIRECTIVE_CD_FILE", invalid_target.path())
        .args(["new", "--shell"])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&output).contains("shell navigation directive"));
    assert!(invalid_target.change_capsules().is_empty());
    assert_eq!(
        invalid_target.git(["branch", "--format=%(refname:short)"]),
        "main"
    );

    let change = missing_wrapper.create_change(None);
    let output = missing_wrapper
        .grove_from(&change.path)
        .env_remove("GROVE_DIRECTIVE_CD_FILE")
        .arg("remove")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&output).contains("shell integration is not loaded"));
    assert!(change.path.exists());
    assert!(missing_wrapper.branch_exists(&change.branch));

    let change = invalid_target.create_change(None);
    let output = invalid_target
        .grove_from(&change.path)
        .env("GROVE_DIRECTIVE_CD_FILE", invalid_target.path())
        .arg("remove")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&output).contains("shell navigation directive"));
    assert!(change.path.exists());
    assert!(invalid_target.branch_exists(&change.branch));

    for shell in ["fish", "zsh"] {
        let shell_repo = TestRepo::new();
        let change = shell_repo.create_change(None);
        shell_repo.set_change_title(&change, "Navigate With Shell");
        let output = shell_repo.switch_from_shell_in_pty(
            shell_repo.path(),
            shell,
            "grove switch --shell",
            "Navigate With Shell",
            b"\r",
        );
        assert!(output.status.success(), "{shell}: {output:?}");
        let terminal = stdout(&output);
        let expected = change.path.canonicalize().unwrap();
        assert!(
            terminal.contains(&format!("__PWD__{}", expected.display())),
            "{shell}: {terminal}"
        );
        assert_terminal_restored(&terminal);

        let output = shell_repo.switch_from_shell_in_pty(
            &change.path,
            shell,
            "grove switch --shell",
            "Navigate With Shell",
            b"x\r\x1b",
        );
        assert!(output.status.success(), "{shell}: {output:?}");
        let terminal = stdout(&output);
        let expected = shell_repo.path().canonicalize().unwrap();
        assert!(terminal.contains("Main repository"), "{shell}: {terminal}");
        assert!(
            terminal.contains(&format!("__PWD__{}", expected.display())),
            "{shell}: {terminal}"
        );
        assert_terminal_restored(&terminal);

        let output = shell_repo.switch_from_shell_in_pty(
            &change.path,
            shell,
            "grove switch --shell",
            "Navigate With Shell",
            b"\x1b[B\r",
        );
        assert!(output.status.success(), "{shell}: {output:?}");
        let terminal = stdout(&output);
        let expected = change.path.canonicalize().unwrap();
        assert!(
            terminal.contains(&format!("__PWD__{}", expected.display())),
            "{shell}: {terminal}"
        );
        assert_terminal_restored(&terminal);

        let output = shell_repo.switch_from_shell_in_pty(
            &change.path,
            shell,
            "grove switch",
            "Navigate With Shell",
            b"\r",
        );
        assert!(output.status.success(), "{shell}: {output:?}");
        let terminal = stdout(&output);
        let expected = shell_repo.path().canonicalize().unwrap();
        assert!(
            terminal.contains(&format!("__PWD__{}", expected.display())),
            "{shell}: {terminal}"
        );
        assert_eq!(shell_repo.agent_log(), "");
        assert_terminal_restored(&terminal);
    }

    let commands = repo
        .grove()
        .env("COMPLETE", "fish")
        .args(["--", "grove", ""])
        .assert()
        .success()
        .get_output()
        .clone();
    let commands = stdout(&commands);
    assert!(commands.contains("switch\t"), "{commands}");
    assert!(commands.contains("remove\t") || commands.contains("delete\t"));
    let flags = repo
        .grove()
        .env("COMPLETE", "fish")
        .args(["--", "grove", "switch", "--"])
        .assert()
        .success()
        .get_output()
        .clone();
    let flags = stdout(&flags);
    assert!(flags.contains("--shell"), "{flags}");
    assert!(!flags.contains("manual-name"), "{flags}");
}

#[test]
fn id_capsules_record_bases_rollback_and_repository_isolation() {
    let repo = TestRepo::new();
    repo.remove_pi();
    let original = repo.git(["rev-parse", "main"]);
    let repository_root = repo.home().join(".grove/repositories");
    repo.grove().args(["new", "--shell"]).assert().success();

    let capsule = repo.change_capsules().pop().expect("created capsule");
    let id = capsule.file_name().unwrap().to_str().unwrap();
    assert_eq!(id.len(), 8);
    assert!(
        id.bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    let repository = capsule.parent().expect("repository directory");
    let repository_name = repository.file_name().unwrap().to_string_lossy();
    assert!(repository_name.starts_with("repo-"), "{repository_name}");
    assert_eq!(repository_name.len(), "repo-12345678".len());
    assert_eq!(
        repository.parent().unwrap(),
        repo.home().join(".grove/repositories")
    );
    assert!(!repository_root.join(".lock").exists());
    let repository_record = repo.repository_record(repository);
    assert_eq!(repository_record["version"], 1);
    assert_eq!(repository_record["name"], "repo");
    assert_eq!(
        Path::new(repository_record["git_common_dir"].as_str().unwrap()),
        repo.path().join(".git").canonicalize().unwrap()
    );
    let record = repo.change_record(&capsule);
    assert_eq!(record["version"], 1);
    assert_eq!(record["id"], id);
    assert_eq!(record["state"], "active");
    assert_eq!(record["title"], serde_json::Value::Null);
    assert_eq!(record["creation"]["base_oid"], original);
    assert_eq!(record["creation"]["base_ref"], serde_json::Value::Null);
    assert_eq!(record["creation"]["parent"], "main");
    assert_eq!(repo.navigation(), capsule.join("worktree"));
    assert_eq!(
        repo.git_from(&repo.navigation(), ["branch", "--show-current"]),
        id
    );
    assert_eq!(repo.agent_log(), "");
    assert!(!repo.grove_runtime_exists());
    assert_eq!(
        fs::metadata(&capsule).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(capsule.join("change.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    repo.commit_file(repo.path(), "second.txt", "second\n");
    let head = repo.git(["rev-parse", "main"]);
    let parent = repo.git(["rev-parse", "main^"]);
    repo.git(["tag", "release", &parent]);
    for (source, oid, parent_name) in [
        ("main", head.as_str(), Some("main")),
        ("release", parent.as_str(), None),
        ("main^", parent.as_str(), None),
        ("@", head.as_str(), Some("main")),
    ] {
        let change = repo.create_change(Some(source));
        assert_eq!(repo.git(["rev-parse", &change.branch]), oid);
        let record = repo.change_record(change.path.parent().unwrap());
        assert_eq!(record["creation"]["base_ref"], source);
        assert_eq!(record["creation"]["base_oid"], oid);
        assert_eq!(
            record["creation"]["parent"],
            parent_name
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null)
        );
    }

    let before_branches = repo.git(["branch", "--format=%(refname:short)"]);
    let before_worktrees = repo.git(["worktree", "list", "--porcelain"]);
    repo.grove()
        .args(["new", "--shell", "--from", "does-not-exist"])
        .assert()
        .failure();
    repo.grove()
        .args(["new", "--shell", "--from", "HEAD:README.md"])
        .assert()
        .failure();
    assert_eq!(
        repo.git(["branch", "--format=%(refname:short)"]),
        before_branches
    );
    assert_eq!(
        repo.git(["worktree", "list", "--porcelain"]),
        before_worktrees
    );

    let other = repo.create_repo("other/repo");
    let first = repo.create_change(None);
    let second = repo.create_change_from(&other, None);
    assert_ne!(first.path, second.path);
    assert_ne!(
        first.path.parent().and_then(std::path::Path::parent),
        second.path.parent().and_then(std::path::Path::parent)
    );
    assert_eq!(
        repo.git_from(&second.path, ["branch", "--show-current"]),
        second.branch
    );
    let first_repository = first.path.parent().unwrap().parent().unwrap();
    let second_repository = second.path.parent().unwrap().parent().unwrap();
    let first_name = first_repository.file_name().unwrap().to_str().unwrap();
    assert!(first_name.starts_with("repo-"), "{first_name}");
    assert_eq!(first_name.len(), "repo-".len() + 8);
    let collision_name = second_repository.file_name().unwrap().to_str().unwrap();
    assert!(collision_name.starts_with("repo-"), "{collision_name}");
    assert_eq!(collision_name.len(), "repo-".len() + 8);
    assert_ne!(first_name, collision_name);
    assert_eq!(
        Path::new(
            repo.repository_record(second_repository)["git_common_dir"]
                .as_str()
                .unwrap()
        ),
        other.join(".git").canonicalize().unwrap()
    );

    let readable = repo.create_repo("named/Project Name");
    let readable_change = repo.create_change_from(&readable, None);
    let readable_name = readable_change
        .path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy();
    assert!(
        readable_name.starts_with("Project Name-"),
        "{readable_name}"
    );
    assert_eq!(readable_name.len(), "Project Name-12345678".len());

    let blocked = TestRepo::new();
    fs::write(blocked.home().join(".grove"), "not a directory").unwrap();
    blocked.grove().args(["new", "--shell"]).assert().failure();
    assert_eq!(blocked.git(["branch", "--format=%(refname:short)"]), "main");
    assert!(blocked.change_capsules().is_empty());

    let failed = TestRepo::new();
    let metadata = failed.path().join(".git/worktrees");
    fs::write(&metadata, "blocked\n").unwrap();
    let error = failed
        .grove()
        .args(["new", "--shell"])
        .assert()
        .failure()
        .get_output()
        .clone();
    fs::remove_file(metadata).unwrap();
    assert!(stderr(&error).contains("could not create change worktree"));
    assert_eq!(failed.git(["branch", "--format=%(refname:short)"]), "main");
    assert!(failed.change_capsules().is_empty());
}

#[test]
fn native_pi_create_resume_lock_failure_and_titles_are_one_workflow() {
    let repo = TestRepo::new();
    let gate = repo.block_title_generator();
    repo.grove()
        .arg("new")
        .env(
            "GROVE_TEST_AGENT_PROMPT",
            "Please implement native session title inference",
        )
        .env("GROVE_TEST_TITLE", "Implement Native Session Titles")
        .env("GROVE_TEST_TITLE_BLOCK", &gate)
        .assert()
        .success();

    let capsule = repo.change_capsules().pop().unwrap();
    let worktree = capsule.join("worktree");
    let sessions = capsule.join("sessions/pi");
    assert_eq!(
        repo.change_record(&capsule)["title"],
        serde_json::Value::Null
    );
    assert!(gate.exists(), "interactive Pi waited for naming");
    repo.wait_for_agent_log("arg=<--system-prompt>");
    let log = repo.agent_log();
    assert!(log.contains("mode=interactive"), "{log}");
    assert!(
        log.contains(&format!(
            "cwd={}",
            worktree.canonicalize().unwrap().display()
        )),
        "{log}"
    );
    assert!(
        log.contains(&format!("arg=<{}>", sessions.display())),
        "{log}"
    );
    for flag in [
        "--session-dir",
        "--continue",
        "--extension",
        "--print",
        "--no-session",
        "--no-tools",
        "--no-context-files",
        "--no-skills",
        "--no-extensions",
        "--system-prompt",
    ] {
        assert!(log.contains(&format!("arg=<{flag}>")), "{log}");
    }
    assert!(!repo.navigation_exists());

    repo.release_title_generator(&gate);
    repo.wait_for_change_title(&capsule, "Implement Native Session Titles");
    repo.wait_for_session_content(r#""name":"Implement Native Session Titles""#);
    assert!(capsule.join(".activity.lock").is_file());
    assert!(capsule.join(".metadata.lock").is_file());
    assert!(!capsule.join(".lock").exists());
    assert!(!capsule.join(".record.lock").exists());
    let session_path = repo.pi_session_files().pop().unwrap();
    let session_before = fs::read_to_string(&session_path).unwrap();
    let id = capsule.file_name().unwrap().to_string_lossy();
    assert_eq!(session_before.matches(r#""customType":"grove""#).count(), 1);
    assert!(session_before.contains(r#""schema":1"#));
    assert!(session_before.contains(&format!(r#""changeId":"{id}""#)));

    let resumed = repo.select_agent_in_pty("Implement Native Session Titles", b"\r");
    assert!(resumed.status.success(), "{resumed:?}");
    assert_eq!(repo.agent_log().matches("mode=interactive").count(), 2);
    assert_eq!(repo.pi_session_files().len(), 1);
    assert_eq!(fs::read_to_string(&session_path).unwrap(), session_before);
    assert!(!repo.navigation_exists());

    let second_title = repo
        .grove_from(&worktree)
        .args(["__title", "--change", &id, "--session", "second-session"])
        .env("GROVE_CHANGE_CAPSULE", &capsule)
        .env("GROVE_TEST_TITLE", "Name A Later Session")
        .write_stdin("A later Pi session has a different purpose")
        .assert()
        .success()
        .get_output()
        .clone();
    assert_eq!(stdout(&second_title).trim(), "Name A Later Session");
    assert_eq!(
        repo.change_record(&capsule)["title"],
        "Implement Native Session Titles"
    );
    assert_eq!(repo.git_from(&worktree, ["branch", "--show-current"]), id);
    assert_eq!(worktree, capsule.join("worktree"));

    let best_effort = TestRepo::new();
    best_effort
        .grove()
        .arg("new")
        .env("GROVE_TEST_AGENT_PROMPT", "This naming request fails")
        .env("GROVE_TEST_TITLE_EXIT", "17")
        .assert()
        .success();
    best_effort.wait_for_agent_log("arg=<--system-prompt>");
    let unnamed = best_effort.change_capsules().pop().unwrap();
    assert_eq!(
        best_effort.change_record(&unnamed)["title"],
        serde_json::Value::Null
    );
    best_effort
        .grove_from(&unnamed.join("worktree"))
        .args([
            "__title",
            "--change",
            &unnamed.file_name().unwrap().to_string_lossy(),
            "--session",
            "malformed",
        ])
        .env("GROVE_CHANGE_CAPSULE", &unnamed)
        .env("GROVE_TEST_TITLE", "Only Two")
        .write_stdin("Generate an invalid title")
        .assert()
        .failure();
    assert_eq!(
        best_effort.change_record(&unnamed)["title"],
        serde_json::Value::Null
    );

    let missing = TestRepo::new();
    missing.remove_pi();
    let before = missing.git(["worktree", "list", "--porcelain"]);
    let error = missing
        .grove()
        .arg("new")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&error).contains("Pi executable"));
    assert!(missing.change_capsules().is_empty());
    assert_eq!(missing.git(["worktree", "list", "--porcelain"]), before);

    let failed = TestRepo::new();
    let error = failed
        .grove()
        .arg("new")
        .env("GROVE_TEST_AGENT_EXIT", "23")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&error).contains("Pi exited with exit status: 23"));
    let retained = failed.change_capsules().pop().unwrap();
    assert!(retained.join("worktree").exists());
    assert!(failed.branch_exists(retained.file_name().unwrap().to_str().unwrap()));
    assert_eq!(failed.pi_session_files().len(), 1);

    let locked = TestRepo::new();
    let (agent, lock_gate) = locked.start_blocking_new();
    let locked_capsule = locked.change_capsules().pop().unwrap();
    let locked_worktree = locked_capsule.join("worktree");
    assert!(locked_capsule.join(".activity.lock").is_file());
    assert!(!locked_capsule.join(".lock").exists());
    let switch = locked.select_agent_in_pty("Untitled", b"\r");
    assert!(!switch.status.success());
    assert!(
        stdout(&switch).contains("already open"),
        "{}",
        stdout(&switch)
    );
    let remove = locked
        .grove_from(&locked_worktree)
        .args(["remove", "--force"])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&remove).contains("already open"));
    assert!(locked_worktree.exists());
    locked.release_blocking_agent(agent, &lock_gate);
    locked
        .grove_from(&locked_worktree)
        .args(["remove", "--force"])
        .assert()
        .success();
}

#[test]
fn title_first_list_and_picker_exclude_unmanaged_and_fail_safely() {
    let repo = TestRepo::new();
    let mut changes = [
        repo.create_change(None),
        repo.create_change(None),
        repo.create_change(None),
        repo.create_change(None),
    ];
    changes.sort_by(|left, right| left.branch.cmp(&right.branch));
    let named = &changes[0];
    let duplicate = &changes[1];
    let untitled = &changes[2];
    let fourth = &changes[3];
    repo.set_change_title(named, "Capture Native Sessions");
    repo.set_change_title(duplicate, "Capture Native Sessions");
    repo.set_change_title(fourth, "Review Active Changes");
    let ordinary = repo.home().join("ordinary");
    repo.git(["branch", "ordinary"]);
    repo.git(["worktree", "add", ordinary.to_str().unwrap(), "ordinary"]);
    let detached = repo.home().join("detached");
    repo.git(["worktree", "add", "--detach", detached.to_str().unwrap()]);

    let listed = repo
        .grove()
        .arg("list")
        .assert()
        .success()
        .get_output()
        .clone();
    let table = stdout(&listed);
    assert!(table.contains("Title"), "{table}");
    assert!(!table.contains("Branch"), "{table}");
    assert!(table.contains(&format!("Capture Native Sessions · {}", &named.branch[..8])));
    assert!(table.contains(&format!(
        "Capture Native Sessions · {}",
        &duplicate.branch[..8]
    )));
    assert!(table.contains(&format!("Untitled · {}", &untitled.branch[..8])));
    assert!(
        !table.contains("ordinary") && !table.contains("detached"),
        "{table}"
    );
    assert!(table.contains("Review Active Changes"), "{table}");
    assert!(stderr(&listed).contains("Showing 4 changes"));

    let selected = repo.switch_in_pty(repo.path(), "Capture Native Sessions", b"\r");
    assert!(selected.status.success(), "{selected:?}");
    let terminal = stdout(&selected);
    assert!(!terminal.contains('⌕'), "{terminal}");
    assert!(terminal.contains("✓ Using"), "{terminal}");
    assert!(!terminal.contains("ordinary") && !terminal.contains("detached"));
    let selected_path = repo.navigation();
    assert!(
        changes
            .iter()
            .any(|change| change.path.canonicalize().unwrap() == selected_path),
        "{}",
        selected_path.display()
    );
    assert_terminal_restored(&terminal);

    let before = repo.navigation();
    let unmanaged = repo.switch_in_pty(&ordinary, "Capture Native Sessions", b"\x1b");
    assert!(unmanaged.status.success(), "{unmanaged:?}");
    let terminal = stdout(&unmanaged);
    assert!(!terminal.contains("Main repository"), "{terminal}");
    assert!(!terminal.contains("Error:"), "{terminal}");
    assert_eq!(repo.navigation(), before);
    assert_terminal_restored(&terminal);

    for input in [b"\x1b".as_slice(), b"\x03".as_slice()] {
        let before = repo.navigation();
        let cancelled = repo.switch_in_pty(repo.path(), "Capture Native Sessions", input);
        assert!(cancelled.status.success(), "{cancelled:?}");
        let terminal = stdout(&cancelled);
        assert!(!terminal.contains("Error:"), "{terminal}");
        assert_eq!(repo.navigation(), before);
        assert_terminal_restored(&terminal);
    }

    let non_tty = repo
        .grove()
        .arg("switch")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&non_tty).contains("interactive worktree selection requires a terminal"));
    let non_tty = repo
        .grove()
        .arg("remove")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&non_tty).contains("interactive worktree selection requires a terminal"));
    assert!(named.path.exists() && repo.branch_exists(&named.branch));

    let removable = TestRepo::new();
    let change = removable.create_change(None);
    removable.set_change_title(&change, "Remove Finished Change");
    let cancelled = removable.remove_in_pty("Remove Finished Change", b"\x1b");
    assert!(cancelled.status.success(), "{cancelled:?}");
    let terminal = stdout(&cancelled);
    assert!(!terminal.contains("Error:"), "{terminal}");
    assert!(change.path.exists() && removable.branch_exists(&change.branch));
    assert_terminal_restored(&terminal);

    let removed = removable.remove_in_pty("Remove Finished Change", b"\r");
    assert!(removed.status.success(), "{removed:?}");
    let terminal = stdout(&removed);
    assert!(
        terminal.contains("✓ Removed Remove Finished Change"),
        "{terminal}"
    );
    assert_terminal_restored(&terminal);
    assert!(!change.path.exists() && !removable.branch_exists(&change.branch));

    let corrupt = TestRepo::new();
    let change = corrupt.create_change(None);
    fs::write(
        change.path.parent().unwrap().join("change.json"),
        "not json\n",
    )
    .unwrap();
    let error = corrupt
        .grove()
        .arg("list")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&error).contains("invalid change record"));
    assert!(change.path.exists() && corrupt.branch_exists(&change.branch));

    let empty = TestRepo::new();
    let listed = empty
        .grove()
        .arg("list")
        .assert()
        .success()
        .get_output()
        .clone();
    let table = stdout(&listed);
    assert!(table.contains("Title"));
    assert!(table.contains("@ Main repository"), "{table}");
    assert!(stderr(&listed).contains("Showing 0 changes"));
    let error = empty
        .grove()
        .arg("switch")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&error).contains("no active changes to switch to"));
}

#[test]
fn terminal_tables_stay_within_the_terminal_width() {
    let repo = TestRepo::new();
    let first = repo.create_change(None);
    let second = repo.create_change(None);
    repo.set_change_title(&first, "First Narrow Picker Change");
    repo.set_change_title(&second, "Second Narrow Picker Change");
    let full_path = format!(
        "~/{}",
        first.path.strip_prefix(repo.home()).unwrap().display()
    );

    let listed = repo.list_in_narrow_pty();
    assert!(listed.status.success(), "{listed:?}");
    let terminal = stdout(&listed);
    assert!(!terminal.contains(&full_path), "{terminal}");
    assert!(terminal.contains('…'), "{terminal}");

    let redirected = repo.grove().arg("list").output().unwrap();
    assert!(redirected.status.success(), "{redirected:?}");
    assert!(stdout(&redirected).contains(&full_path));

    let removed = repo.remove_in_narrow_pty("First Narrow Picker Change", b"\x1b[B\x1b");
    assert!(removed.status.success(), "{removed:?}");
    let terminal = stdout(&removed);
    assert!(!terminal.contains(&full_path), "{terminal}");
    assert!(first.path.exists() && second.path.exists());
    assert_terminal_restored(&terminal);
}

#[test]
fn sync_fetches_exact_upstream_archives_and_rebases_safely() {
    let repo = TestRepo::new();
    let publisher = repo.create_local_origin();
    let stale_main = repo.git(["rev-parse", "main"]);

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
    repo.commit_file(&integrated.path, "integrated.txt", "integrated remotely\n");
    let integrated_tip = repo.git(["rev-parse", &integrated.branch]);

    let remaining = repo.create_change(Some("main"));
    repo.commit_file(&remaining.path, "change.txt", "local change\n");

    let reapplied = repo.create_change(Some("main"));
    let reapplied_tip = repo.commit_file(
        &reapplied.path,
        "reapplied.txt",
        "content that must survive sync\n",
    );

    repo.git(["config", "--global", "rebase.updateRefs", "true"]);
    let protected = repo.create_change(Some("main"));
    let intermediate = repo.commit_file(&protected.path, "first.txt", "first change\n");
    repo.commit_file(&protected.path, "second.txt", "second change\n");
    repo.git(["branch", "unmanaged-snapshot", &intermediate]);

    repo.commit_file(&publisher, "prelude.txt", "upstream prelude\n");
    repo.git_from(
        &publisher,
        ["fetch", repo.path().to_str().unwrap(), &reapplied.branch],
    );
    repo.git_from(&publisher, ["cherry-pick", &reapplied_tip]);
    assert_ne!(
        repo.git_from(&publisher, ["rev-parse", "HEAD"]),
        reapplied_tip
    );
    repo.git_from(&publisher, ["revert", "--no-edit", "HEAD"]);
    repo.git_from(
        &publisher,
        ["fetch", repo.path().to_str().unwrap(), &integrated.branch],
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
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "✓ Synced: 1 archived, 3 rebased, 0 skipped\n"
    );
    assert_eq!(repo.git(["rev-parse", "main"]), stale_main);
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
    assert!(!integrated.path.exists() && !repo.branch_exists(&integrated.branch));
    assert!(integrated_capsule.join("artifacts/change.patch").is_file());
    let record = repo.change_record(integrated_capsule);
    assert_eq!(record["state"], "archived");
    assert_eq!(record["closure"]["tip_oid"], integrated_tip);
    assert_eq!(record["closure"]["target_oid"], fetched_upstream);
    assert_eq!(record["closure"]["integration"], "merge-equivalent");

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
    let other_tip = repo.commit_file(&other_parent.path, "release.txt", "release change\n");

    let rewritten = repo.create_change(Some("main"));
    let rewritten_tip = repo.git(["rev-parse", "main^"]);
    repo.git_from(&rewritten.path, ["reset", "--hard", &rewritten_tip]);

    repo.git(["checkout", "-b", "merge-side", "main"]);
    repo.commit_file(repo.path(), "side.txt", "side work\n");
    repo.git(["checkout", "main"]);
    let merged = repo.create_change(Some("main"));
    repo.commit_file(&merged.path, "change.txt", "change work\n");
    repo.git_from(
        &merged.path,
        ["merge", "--no-ff", "merge-side", "-m", "Merge side"],
    );
    let merged_tip = repo.git(["rev-parse", &merged.branch]);

    repo.commit_file(&publisher, "upstream.txt", "upstream work\n");
    repo.git_from(&publisher, ["push", "origin", "main"]);
    let output = repo.grove().arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stderr(&output),
        "✓ Synced: 0 archived, 0 rebased, 3 skipped\n"
    );
    for (change, tip) in [
        (&other_parent, other_tip),
        (&rewritten, rewritten_tip),
        (&merged, merged_tip),
    ] {
        assert_eq!(repo.git(["rev-parse", &change.branch]), tip);
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
    let tip = diverged.commit_file(&change.path, "change.txt", "local change\n");
    diverged.git_from(&publisher, ["reset", "--hard", &format!("{base}^")]);
    diverged.commit_file(&publisher, "replacement.txt", "replacement history\n");
    diverged.git_from(&publisher, ["push", "--force", "origin", "HEAD:main"]);
    let output = diverged.grove().arg("sync").output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stderr(&output),
        "✓ Synced: 0 archived, 0 rebased, 1 skipped\n"
    );
    assert_eq!(diverged.git(["rev-parse", &change.branch]), tip);
    assert_eq!(diverged.git(["rev-parse", "main"]), base);
}

#[test]
fn sync_validation_and_fetch_failures_happen_before_mutation() {
    {
        let repo = TestRepo::new();
        repo.create_local_origin();
        let change = repo.create_change(Some("main"));
        let content_path = change.path.join("change.txt");
        repo.commit_file(&change.path, "change.txt", "committed change\n");

        let capsule = change.path.parent().unwrap();
        let branch_before = repo.git(["rev-parse", &change.branch]);
        let status_before = repo.git_from(&change.path, ["status", "--porcelain=v1"]);
        let content_before = fs::read(&content_path).unwrap();
        let record_before = fs::read(capsule.join("change.json")).unwrap();
        let artifacts = capsule.join("artifacts");
        assert_eq!(repo.change_record(capsule)["state"], "active");
        assert!(!artifacts.exists());

        let origin = repo.git(["remote", "get-url", "origin"]);
        fs::remove_dir_all(origin).unwrap();
        let output = repo.grove().arg("sync").output().unwrap();

        assert!(!output.status.success(), "{output:?}");
        let error = stderr(&output);
        assert!(error.contains("failed to fetch merge ref"), "{error}");
        assert!(repo.branch_exists(&change.branch));
        assert_eq!(repo.git(["rev-parse", &change.branch]), branch_before);
        assert_eq!(
            repo.git_from(&change.path, ["status", "--porcelain=v1"]),
            status_before
        );
        assert_eq!(fs::read(content_path).unwrap(), content_before);
        assert_eq!(
            fs::read(capsule.join("change.json")).unwrap(),
            record_before
        );
        assert!(!artifacts.exists());
    }

    {
        let repo = TestRepo::new();
        let publisher = repo.create_local_origin();
        let stale_upstream = repo.git(["rev-parse", "refs/remotes/origin/main"]);

        let change = repo.create_change(Some("main"));
        repo.commit_file(&change.path, "change.txt", "integrated remotely\n");
        repo.git_from(
            &publisher,
            ["fetch", repo.path().to_str().unwrap(), &change.branch],
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

        let branch_before = repo.git(["rev-parse", &change.branch]);
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
        assert!(repo.branch_exists(&change.branch));
        assert_eq!(repo.git(["rev-parse", &change.branch]), branch_before);
        assert_eq!(worktree_bytes(&change.path), worktree_before);
        assert_eq!(fs::read(record_path).unwrap(), record_before);
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
    assert_eq!(stdout(&output), "");
    let summary = stderr(&output);
    assert_eq!(summary.lines().count(), 1, "{summary}");
    assert_eq!(summary, "✓ Synced: 0 archived, 1 rebased, 2 skipped\n");

    assert_eq!(repo.git(["rev-parse", "main"]), stale_main);
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
        branch: busy_capsule
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        path: busy_capsule.join("worktree"),
    };
    let busy_tip = repo.commit_file(&busy.path, "busy.txt", "busy change\n");

    let locked = repo.create_change(Some("main"));
    let locked_tip = repo.commit_file(&locked.path, "locked.txt", "locked change\n");

    let missing = repo.create_change(Some("main"));
    let missing_tip = repo.commit_file(&missing.path, "missing.txt", "missing change\n");
    let missing_git_dir =
        Path::new(&repo.git_from(&missing.path, ["rev-parse", "--absolute-git-dir"])).to_owned();

    let eligible = repo.create_change(Some("main"));
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
    assert_eq!(stdout(&output), "");
    let summary = stderr(&output);
    assert_eq!(summary.lines().count(), 1, "{summary}");
    assert_eq!(summary, "✓ Synced: 0 archived, 1 rebased, 3 skipped\n");

    for ((change, record_before), tip_before) in
        skipped
            .iter()
            .zip(&records_before)
            .zip([busy_tip, locked_tip, missing_tip])
    {
        assert!(repo.branch_exists(&change.branch));
        assert_eq!(repo.git(["rev-parse", &change.branch]), tip_before);
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

    assert!(repo.branch_exists(&eligible.branch));
    assert_ne!(repo.git(["rev-parse", &eligible.branch]), eligible_tip);
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

#[test]
fn integrated_merge_cherry_pick_and_squash_remove_but_unmerged_work_does_not() {
    let merged = TestRepo::new();
    let change = merged.create_change(None);
    merged.commit_file(&change.path, "merged.txt", "merged\n");
    merged.git(["merge", "--no-ff", "-m", "Merge change", &change.branch]);
    merged
        .grove_from(&change.path)
        .arg("remove")
        .assert()
        .success();
    assert!(!merged.branch_exists(&change.branch) && !change.path.exists());

    let cherry_picked = TestRepo::new();
    let change = cherry_picked.create_change(None);
    let tip = cherry_picked.commit_file(&change.path, "picked.txt", "picked\n");
    cherry_picked.git(["cherry-pick", &tip]);
    cherry_picked
        .grove_from(&change.path)
        .arg("delete")
        .assert()
        .success();
    assert!(!cherry_picked.branch_exists(&change.branch) && !change.path.exists());

    let squashed = TestRepo::new();
    let change = squashed.create_change(None);
    squashed.commit_file(&change.path, "one.txt", "one\n");
    squashed.commit_file(&change.path, "two.txt", "two\n");
    squashed.git(["merge", "--squash", &change.branch]);
    squashed.git(["commit", "-m", "Squash change"]);
    squashed
        .grove_from(&change.path)
        .arg("remove")
        .assert()
        .success();
    assert!(!squashed.branch_exists(&change.branch) && !change.path.exists());

    let raced = TestRepo::new();
    let change = raced.create_change(None);
    commit_race_files(&raced, &change, "target", 100);
    let base = raced.git(["rev-parse", "main"]);
    raced.git([
        "merge",
        "--no-ff",
        "-m",
        "Merge race fixture",
        &change.branch,
    ]);
    let integrated_tip = raced.git(["rev-parse", "main"]);
    let capsule = change.path.parent().unwrap();
    let child = raced.spawn_grove_from(&change.path, ["remove"]);
    wait_for_archive_start(capsule);
    raced.git(["update-ref", "refs/heads/main", &base, &integrated_tip]);
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("integration target 'main' changed"),
        "{}",
        stderr(&output)
    );
    assert!(change.path.exists() && raced.branch_exists(&change.branch));
    assert_eq!(raced.change_record(capsule)["state"], "closing");
    raced.git(["update-ref", "refs/heads/main", &integrated_tip, &base]);
    raced
        .grove_from(&change.path)
        .arg("remove")
        .assert()
        .success();

    let branch_raced = TestRepo::new();
    let change = branch_raced.create_change(None);
    commit_race_files(&branch_raced, &change, "branch", 50);
    branch_raced.git([
        "merge",
        "--no-ff",
        "-m",
        "Merge branch race fixture",
        &change.branch,
    ]);
    let expected_tip = branch_raced.git(["rev-parse", &change.branch]);
    let tree = branch_raced.git(["rev-parse", &format!("{}^{{tree}}", change.branch)]);
    let changed_tip = branch_raced.git([
        "commit-tree",
        &tree,
        "-p",
        &expected_tip,
        "-m",
        "Concurrent branch update",
    ]);
    let capsule = change.path.parent().unwrap();
    let child = branch_raced.spawn_grove_from(&change.path, ["remove"]);
    wait_for_archive_start(capsule);
    branch_raced.git([
        "update-ref",
        &format!("refs/heads/{}", change.branch),
        &changed_tip,
        &expected_tip,
    ]);
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(
        stderr(&output).contains("changed before it could be deleted"),
        "{}",
        stderr(&output)
    );
    assert!(change.path.exists());
    assert_eq!(branch_raced.git(["rev-parse", &change.branch]), changed_tip);
    assert_eq!(branch_raced.change_record(capsule)["state"], "closing");
    branch_raced
        .grove_from(&change.path)
        .arg("remove")
        .assert()
        .success();

    let unmerged = TestRepo::new();
    let change = unmerged.create_change(None);
    unmerged.commit_file(&change.path, "unmerged.txt", "unmerged\n");
    let error = unmerged
        .grove_from(&change.path)
        .arg("remove")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&error).contains("not merged"), "{}", stderr(&error));
    assert!(unmerged.branch_exists(&change.branch) && change.path.exists());
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
    let cherry = repo.git(["cherry", "main", &change.branch]);
    assert!(
        !cherry.is_empty() && cherry.lines().all(|line| line.starts_with('-')),
        "{cherry}"
    );

    let error = repo
        .grove_from(worktree)
        .arg("remove")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&error).contains("not merged"), "{}", stderr(&error));
    assert!(worktree.exists() && repo.branch_exists(&change.branch));
}

#[test]
fn safe_removal_archives_git_facts_preserves_native_sessions_and_excludes_change() {
    let repo = TestRepo::new();
    let change = repo.create_change(Some("main"));
    repo.set_change_title(&change, "Archive Finished Change");
    repo.commit_file(&change.path, "finished.txt", "finished\n");
    repo.git([
        "merge",
        "--no-ff",
        "-m",
        "Merge archived change",
        &change.branch,
    ]);
    let capsule = change.path.parent().unwrap();
    let sessions = capsule.join("sessions/pi");
    fs::create_dir_all(&sessions).unwrap();
    let session = sessions.join("native.jsonl");
    let session_contents = b"{\"type\":\"session\",\"id\":\"native\"}\n";
    fs::write(&session, session_contents).unwrap();

    repo.grove_from(&change.path)
        .arg("remove")
        .assert()
        .success();
    assert_eq!(repo.navigation(), repo.path().canonicalize().unwrap());
    assert!(!change.path.exists() && !repo.branch_exists(&change.branch));
    assert!(capsule.exists());
    assert_eq!(fs::read(&session).unwrap(), session_contents);
    let patch_path = capsule.join("artifacts/change.patch");
    let stats_path = capsule.join("artifacts/stats.json");
    let patch = fs::read_to_string(&patch_path).unwrap();
    assert!(
        patch.contains("finished.txt") && patch.contains("+finished"),
        "{patch}"
    );
    let stats: serde_json::Value = serde_json::from_slice(&fs::read(&stats_path).unwrap()).unwrap();
    assert_eq!(stats["version"], 1);
    assert_eq!(stats["patch_digest"].as_str().map(str::len), Some(64));
    assert_eq!(stats["files"][0]["path"], "finished.txt");
    let record = repo.change_record(capsule);
    assert_eq!(record["state"], "archived");
    assert_eq!(record["closure"]["outcome"], "integrated");
    assert!(record["closure"]["closed_at"].is_number());
    for artifact in [patch_path, stats_path] {
        assert_eq!(
            fs::metadata(artifact).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let listed = repo
        .grove()
        .arg("list")
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(!stdout(&listed).contains("Archive Finished Change"));
    assert!(stderr(&listed).contains("Showing 0 changes"));
}

#[test]
fn force_archives_the_complete_dirty_tree_without_ignored_files_or_index_mutation() {
    let repo = TestRepo::new();
    repo.commit_file(repo.path(), "delete-me.txt", "delete this\n");
    let change = repo.create_change(None);
    let capsule = change.path.parent().unwrap();
    repo.commit_file(&change.path, "committed.txt", "committed\n");
    repo.git_from(&change.path, ["mv", "README.md", "renamed.md"]);
    fs::write(change.path.join("staged.txt"), "staged\n").unwrap();
    repo.git_from(&change.path, ["add", "staged.txt"]);
    fs::write(change.path.join("committed.txt"), "committed\nunstaged\n").unwrap();
    fs::remove_file(change.path.join("delete-me.txt")).unwrap();
    fs::write(change.path.join("binary.bin"), b"binary\0contents\xff").unwrap();
    fs::write(change.path.join("untracked.txt"), "untracked\n").unwrap();
    fs::write(change.path.join(".gitignore"), "ignored.txt\n").unwrap();
    fs::write(change.path.join("ignored.txt"), "must not archive\n").unwrap();
    let index_before = repo.git_from(&change.path, ["diff", "--cached", "--binary"]);

    repo.grove_from(&change.path)
        .args(["remove", "--force"])
        .assert()
        .success();
    let patch = String::from_utf8_lossy(&fs::read(capsule.join("artifacts/change.patch")).unwrap())
        .into_owned();
    for path in [
        "committed.txt",
        "staged.txt",
        "renamed.md",
        "delete-me.txt",
        "binary.bin",
        "untracked.txt",
        ".gitignore",
        "GIT binary patch",
        "unstaged",
    ] {
        assert!(patch.contains(path), "missing {path:?}: {patch}");
    }
    assert!(!patch.contains("diff --git a/ignored.txt"), "{patch}");
    assert!(index_before.contains("rename from README.md") && index_before.contains("staged.txt"));
    let stats: serde_json::Value =
        serde_json::from_slice(&fs::read(capsule.join("artifacts/stats.json")).unwrap()).unwrap();
    assert!(
        stats["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| { file["old_path"] == "README.md" && file["path"] == "renamed.md" })
    );
    assert!(
        stats["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| { file["path"] == "binary.bin" && file["binary"] == true })
    );
    assert_eq!(stats["summary"]["renamed"], 1);
    assert_eq!(stats["summary"]["binary"], 1);
    let record = repo.change_record(capsule);
    assert_eq!(record["state"], "archived");
    assert_eq!(record["closure"]["outcome"], "discarded");
    assert!(!change.path.exists() && !repo.branch_exists(&change.branch));
}

#[test]
fn artifact_failure_leaves_git_record_and_real_index_untouched() {
    let repo = TestRepo::new();
    let change = repo.create_change(None);
    repo.commit_file(&change.path, "finished.txt", "finished\n");
    repo.git([
        "merge",
        "--no-ff",
        "-m",
        "Merge blocked archive",
        &change.branch,
    ]);
    let capsule = change.path.parent().unwrap();
    fs::write(capsule.join("artifacts"), "block archive install\n").unwrap();
    fs::write(change.path.join("staged.txt"), "staged\n").unwrap();
    repo.git_from(&change.path, ["add", "staged.txt"]);
    let index_before = repo.git_from(&change.path, ["diff", "--cached", "--binary"]);
    let branch_before = repo.git(["rev-parse", &change.branch]);

    let error = repo
        .grove_from(&change.path)
        .args(["remove", "--force"])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&error).contains("artifacts"), "{}", stderr(&error));
    assert!(change.path.exists());
    assert_eq!(repo.git(["rev-parse", &change.branch]), branch_before);
    assert_eq!(
        repo.git_from(&change.path, ["diff", "--cached", "--binary"]),
        index_before
    );
    assert_eq!(repo.change_record(capsule)["state"], "active");

    let interrupted = TestRepo::new();
    let change = interrupted.create_change(None);
    interrupted.commit_file(&change.path, "finished.txt", "finished\n");
    interrupted.git([
        "merge",
        "--no-ff",
        "-m",
        "Merge interrupted cleanup",
        &change.branch,
    ]);
    let capsule = change.path.parent().unwrap();
    let hook = interrupted.path().join(".git/hooks/reference-transaction");
    fs::write(&hook, "#!/bin/sh\ntest \"$1\" != prepared\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();

    let error = interrupted
        .grove_from(&change.path)
        .arg("remove")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(
        stderr(&error).contains("branch cleanup"),
        "{}",
        stderr(&error)
    );
    assert!(!change.path.exists());
    assert!(interrupted.branch_exists(&change.branch));
    assert_eq!(interrupted.change_record(capsule)["state"], "closing");
    assert!(capsule.join("artifacts/change.patch").is_file());

    fs::remove_file(hook).unwrap();
    let recovered = interrupted
        .grove()
        .arg("remove")
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(
        stderr(&recovered).contains("Finished 1 interrupted removal"),
        "{}",
        stderr(&recovered)
    );
    assert!(!interrupted.branch_exists(&change.branch));
    assert_eq!(interrupted.change_record(capsule)["state"], "archived");
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn commit_race_files(repo: &TestRepo, change: &TestChange, prefix: &str, count: usize) {
    for index in 0..count {
        fs::write(
            change.path.join(format!("{prefix}-{index}.txt")),
            format!("{index}\n"),
        )
        .unwrap();
    }
    repo.git_from(&change.path, ["add", "."]);
    repo.git_from(&change.path, ["commit", "-m", "Race fixture"]);
}

fn wait_for_archive_start(capsule: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !fs::read_dir(capsule)
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".artifacts-")
        })
    {
        assert!(Instant::now() < deadline, "archive snapshot did not start");
        thread::sleep(Duration::from_millis(1));
    }
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn assert_terminal_restored(terminal: &str) {
    let flags = terminal.split_whitespace().collect::<Vec<_>>();
    assert!(flags.contains(&"icanon"), "{terminal:?}");
    assert!(flags.contains(&"echo"), "{terminal:?}");
    let hidden = terminal.rfind("\x1b[?25l").expect("picker hides cursor");
    let shown = terminal.rfind("\x1b[?25h").expect("picker restores cursor");
    assert!(hidden < shown, "{terminal:?}");
}
