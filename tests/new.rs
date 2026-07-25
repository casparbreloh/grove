mod support;

use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

use support::{TestRepo, stderr, stdout};

#[test]
fn new_requires_a_commit_backed_default_base() {
    let repo = TestRepo::new();
    let original = repo.git(["rev-parse", "main"]);
    repo.git(["update-ref", "-d", "refs/heads/main"]);
    let worktrees = repo.git(["worktree", "list", "--porcelain"]);

    let output = repo
        .grove()
        .arg("new")
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(
        stderr(&output).contains("create an initial commit or pass --from a commit"),
        "{}",
        stderr(&output)
    );
    assert!(repo.change_capsules().is_empty());
    assert_eq!(repo.git(["worktree", "list", "--porcelain"]), worktrees);

    repo.grove()
        .args(["new", "--from", &original])
        .assert()
        .success();
    let capsule = repo.change_capsules().pop().expect("created capsule");
    assert_eq!(
        repo.git_from(&capsule.join("workspace"), ["rev-parse", "HEAD"]),
        original
    );
    assert_eq!(repo.change_record(&capsule)["base_oid"], original);
}

#[test]
fn id_capsules_record_bases_rollback_and_repository_isolation() {
    let repo = TestRepo::new();
    let original = repo.git(["rev-parse", "main"]);
    let grove_root = repo.home().join(".grove");
    repo.grove().arg("new").assert().success();

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
    assert_eq!(repository.parent().unwrap(), grove_root);
    let record = repo.change_record(&capsule);
    assert_eq!(record["version"], 3);
    assert_eq!(record["id"], id);
    assert_eq!(record["state"], "active");
    assert_eq!(record["title"], serde_json::Value::Null);
    assert_eq!(record["base_oid"], original);
    assert_eq!(record["parent"], "main");
    assert_eq!(record.as_object().unwrap().len(), 7);
    assert!(!repository.join("repository.json").exists());
    assert!(!repo.navigation_exists());
    assert_eq!(
        repo.git_from(&capsule.join("workspace"), ["branch", "--show-current"]),
        ""
    );
    assert_eq!(repo.git(["branch", "--format=%(refname:short)"]), "main");
    assert!(repo.agent_log().contains("mode=interactive"));
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
        assert_eq!(repo.change_head(&change), oid);
        let record = repo.change_record(change.path.parent().unwrap());
        assert_eq!(record["base_oid"], oid);
        assert_eq!(
            record["parent"],
            parent_name
                .map(serde_json::Value::from)
                .unwrap_or(serde_json::Value::Null)
        );
    }

    let before_branches = repo.git(["branch", "--format=%(refname:short)"]);
    let before_worktrees = repo.git(["worktree", "list", "--porcelain"]);
    repo.grove()
        .args(["new", "--from", "does-not-exist"])
        .assert()
        .failure();
    repo.grove()
        .args(["new", "--from", "HEAD:README.md"])
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
        ""
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
    blocked.grove().arg("new").assert().failure();
    assert_eq!(blocked.git(["branch", "--format=%(refname:short)"]), "main");
    assert!(blocked.change_capsules().is_empty());

    let failed = TestRepo::new();
    let metadata = failed.path().join(".git/worktrees");
    fs::write(&metadata, "blocked\n").unwrap();
    let error = failed
        .grove()
        .arg("new")
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
fn pi_extensions_link_sessions_and_validate_structured_output() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("node")
        .arg(root.join("tests/support/extensions.mjs"))
        .arg(root.join("src/extensions/change-session.ts"))
        .arg(root.join("src/extensions/structured-output.ts"))
        .output()
        .expect("Node.js is required to test Pi extensions");
    assert!(
        output.status.success(),
        "Pi extension contract failed:\n{}",
        stderr(&output)
    );
}

#[test]
fn native_pi_create_resume_lock_failure_and_titles_are_one_workflow() {
    let repo = TestRepo::new();
    let gate = repo.block_rpc_worker();
    repo.grove()
        .arg("new")
        .env(
            "GROVE_TEST_AGENT_PROMPT",
            "Please implement native session title inference",
        )
        .env("GROVE_TEST_TITLE", "Implement Native Session Titles")
        .env("GROVE_TEST_RPC_BLOCK", &gate)
        .assert()
        .success();

    let capsule = repo.change_capsules().pop().unwrap();
    let worktree = capsule.join("workspace");
    let sessions = capsule.join("pi");
    assert_eq!(
        repo.change_record(&capsule)["title"],
        serde_json::Value::Null
    );
    assert!(gate.exists(), "interactive Pi waited for naming");
    repo.wait_for_agent_log("arg=<--system-prompt>");
    let log = repo.agent_log();
    assert!(log.contains("mode=interactive"), "{log}");
    let arguments = log
        .lines()
        .filter_map(|line| {
            line.strip_prefix("arg=<")
                .and_then(|argument| argument.strip_suffix('>'))
        })
        .collect::<Vec<_>>();
    let extension = arguments
        .windows(2)
        .find_map(|pair| (pair[0] == "--extension").then_some(pair[1]))
        .expect("managed Pi must receive the Grove extension");
    let extension_name = Path::new(extension).file_name().unwrap().to_string_lossy();
    let extension_hash = extension_name
        .strip_prefix(".grove-session-")
        .and_then(|name| name.strip_suffix(".ts"))
        .expect("Pi extension path must retain its name and TypeScript suffix");
    assert_eq!(extension_hash.len(), 8, "{extension_name}");
    assert!(
        extension_hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "{extension_name}"
    );
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
        "--mode",
        "--no-session",
        "--model",
        "--tools",
        "--structured-output-schema",
        "--no-context-files",
        "--no-skills",
        "--no-extensions",
        "--system-prompt",
    ] {
        assert!(log.contains(&format!("arg=<{flag}>")), "{log}");
    }
    assert!(log.contains("arg=<openai-codex/gpt-5.6-sol>"), "{log}");
    assert!(!repo.navigation_exists());

    repo.release_rpc_worker(&gate);
    repo.wait_for_change_title(&capsule, "Implement Native Session Titles");
    repo.wait_for_session_content(r#""name":"Implement Native Session Titles""#);
    assert!(capsule.join(".activity.lock").is_file());
    assert!(capsule.join(".metadata.lock").is_file());
    assert!(!repo.home().join(".grove/runtime").exists());
    assert!(!capsule.join(".lock").exists());
    assert!(!capsule.join(".record.lock").exists());
    let session_path = repo.pi_session_files().pop().unwrap();
    let session_before = fs::read_to_string(&session_path).unwrap();
    let id = capsule.file_name().unwrap().to_string_lossy();
    assert_eq!(
        session_before
            .matches(r#""customType":"grove.change""#)
            .count(),
        1
    );
    assert!(!session_before.contains(r#""schema":1"#));
    assert!(session_before.contains(&format!(r#""changeId":"{id}""#)));

    let resumed = repo.resume_pi_in_pty("Implement Native Session Titles", b"\x1b[B\r");
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
    assert_eq!(repo.git_from(&worktree, ["branch", "--show-current"]), "");
    assert_eq!(worktree, capsule.join("workspace"));

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
    let workers_before_mismatch = best_effort
        .agent_log()
        .matches("arg=<--system-prompt>")
        .count();
    best_effort
        .grove_from(&unnamed.join("workspace"))
        .args(["__title", "--change", "deadbeef", "--session", "mismatched"])
        .env("GROVE_CHANGE_CAPSULE", &unnamed)
        .write_stdin("Do not send this prompt")
        .assert()
        .failure();
    assert_eq!(
        best_effort
            .agent_log()
            .matches("arg=<--system-prompt>")
            .count(),
        workers_before_mismatch,
        "identity mismatch must fail before starting Pi"
    );
    best_effort
        .grove_from(&unnamed.join("workspace"))
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
    let retry_count = unnamed.join("retry-count");
    let recovered = best_effort
        .grove_from(&unnamed.join("workspace"))
        .args([
            "__title",
            "--change",
            &unnamed.file_name().unwrap().to_string_lossy(),
            "--session",
            "recovered",
        ])
        .env("GROVE_CHANGE_CAPSULE", &unnamed)
        .env("GROVE_TEST_RPC_FAILURES", &retry_count)
        .env("GROVE_TEST_RAW_CHANGE", "Recover Session Naming")
        .write_stdin("Retry transient naming failures")
        .assert()
        .success()
        .get_output()
        .clone();
    assert_eq!(stdout(&recovered).trim(), "Recover Session Naming");
    assert_eq!(fs::read_to_string(&retry_count).unwrap(), "3");
    assert_eq!(
        best_effort.change_record(&unnamed)["title"],
        serde_json::Value::Null,
        "inference alone must not let stale work title the Change"
    );
    best_effort
        .grove_from(&unnamed.join("workspace"))
        .args([
            "__title",
            "--change",
            &unnamed.file_name().unwrap().to_string_lossy(),
            "--session",
            "recovered",
            "--apply",
        ])
        .env("GROVE_CHANGE_CAPSULE", &unnamed)
        .write_stdin("Recover Session Naming")
        .assert()
        .success();
    assert_eq!(
        best_effort.change_record(&unnamed)["title"],
        "Recover Session Naming"
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
    assert!(retained.join("workspace").exists());
    assert!(!failed.branch_exists(retained.file_name().unwrap().to_str().unwrap()));
    assert_eq!(failed.pi_session_files().len(), 1);

    let locked = TestRepo::new();
    let (agent, lock_gate) = locked.start_blocking_new();
    let locked_capsule = locked.change_capsules().pop().unwrap();
    let locked_worktree = locked_capsule.join("workspace");
    assert!(locked_capsule.join(".activity.lock").is_file());
    assert!(!locked_capsule.join(".lock").exists());
    let resumed = locked.resume_pi_in_pty("Untitled", b"\x1b[B\r");
    assert!(!resumed.status.success());
    assert!(
        stdout(&resumed).contains("already open"),
        "{}",
        stdout(&resumed)
    );
    let archive = locked
        .grove_from(&locked_worktree)
        .args(["archive", "--force"])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&archive).contains("already open"));
    assert!(locked_worktree.exists());
    locked.release_blocking_agent(agent, &lock_gate);
    locked
        .grove_from(&locked_worktree)
        .args(["archive", "--force"])
        .assert()
        .success();
}
