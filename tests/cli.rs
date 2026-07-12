use std::{path::Path, process::Command};

use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {:?}: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn repo() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    git(temp.path(), &["init", "-b", "main"]);
    git(temp.path(), &["config", "user.name", "Grove Test"]);
    git(temp.path(), &["config", "user.email", "grove@example.test"]);
    std::fs::write(temp.path().join("README.md"), "hello\n").unwrap();
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "initial"]);
    temp
}

fn expected_worktree(repo: &Path, branch: &str) -> String {
    let root = repo.canonicalize().unwrap();
    let branch = branch
        .bytes()
        .map(|byte| {
            if byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_' | b'.')
            {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect::<String>();
    format!("{}.{}", root.display(), branch)
}

#[test]
fn branch_paths_do_not_collide_after_escaping() {
    let repo = repo();
    let nested = expected_worktree(repo.path(), "feature/login");
    let dashed = expected_worktree(repo.path(), "feature-login");
    assert_ne!(nested, dashed);
    cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature/login"])
        .assert()
        .success();
    cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature-login"])
        .assert()
        .success();
    assert!(Path::new(&nested).exists());
    assert!(Path::new(&dashed).exists());
}

#[test]
fn branch_paths_survive_case_folding_and_unicode_normalization() {
    let repo = repo();
    let cases = [
        ("topic", ".topic"),
        ("Topic", ".%54opic"),
        ("%54opic", ".%2554opic"),
        ("tópic", ".t%C3%B3pic"),
    ];

    let mut paths = Vec::new();
    for (branch, suffix) in cases {
        let output = cargo_bin_cmd!("grove")
            .current_dir(repo.path())
            .args(["switch", "--create", branch])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let path = String::from_utf8(output.stdout).unwrap().trim().to_owned();
        assert!(path.ends_with(suffix), "{path}");
        assert!(Path::new(&path).exists());
        paths.push(path);
        cargo_bin_cmd!("grove")
            .current_dir(repo.path())
            .args(["remove", branch])
            .assert()
            .success();
        git(repo.path(), &["branch", "-D", branch]);
    }
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), 4);
}

#[test]
fn switch_create_uses_default_branch_not_current_topic() {
    let repo = repo();
    git(repo.path(), &["checkout", "-b", "topic"]);
    std::fs::write(repo.path().join("topic.txt"), "topic\n").unwrap();
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-m", "topic"]);

    let output = cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["switch", "--create", "fresh"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let fresh = expected_worktree(repo.path(), "fresh");
    assert_eq!(
        git(Path::new(&fresh), &["rev-parse", "HEAD"]),
        git(repo.path(), &["rev-parse", "main"])
    );
    assert_ne!(
        git(Path::new(&fresh), &["rev-parse", "HEAD"]),
        git(repo.path(), &["rev-parse", "topic"])
    );
}

#[test]
fn switch_create_makes_worktree_from_primary_branch() {
    let repo = repo();
    let output = cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["switch", "--create", "feature/login"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let path = String::from_utf8(output.stdout).unwrap().trim().to_owned();
    assert_eq!(path, expected_worktree(repo.path(), "feature/login"));
    assert_eq!(
        git(Path::new(&path), &["branch", "--show-current"]),
        "feature/login"
    );
    assert_eq!(
        git(Path::new(&path), &["rev-parse", "HEAD"]),
        git(repo.path(), &["rev-parse", "main"])
    );
}

#[test]
fn switch_existing_branch_reuses_or_creates_its_worktree() {
    let repo = repo();
    git(repo.path(), &["branch", "ready"]);
    for _ in 0..2 {
        let output = cargo_bin_cmd!("grove")
            .current_dir(repo.path())
            .args(["switch", "ready"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            expected_worktree(repo.path(), "ready")
        );
    }
}

#[test]
fn switch_without_create_refuses_a_missing_branch() {
    let repo = repo();
    cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["switch", "typo"])
        .assert()
        .failure();
    assert!(!Path::new(&expected_worktree(repo.path(), "typo")).exists());
}

#[test]
fn list_marks_current_primary_and_linked_worktrees() {
    let repo = repo();
    cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["switch", "--create", "topic"])
        .assert()
        .success();
    let topic = expected_worktree(repo.path(), "topic");
    std::fs::write(Path::new(&topic).join("dirty.txt"), "dirty").unwrap();

    let output = cargo_bin_cmd!("grove")
        .current_dir(&topic)
        .arg("list")
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("^ main"), "{stdout}");
    assert!(stdout.contains("@ topic"), "{stdout}");
    assert!(stdout.contains("dirty"), "{stdout}");
    assert!(stdout.contains(repo.path().to_str().unwrap()), "{stdout}");
}

#[test]
fn list_renders_missing_registered_worktree_as_prunable() {
    let repo = repo();
    cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["switch", "--create", "gone"])
        .assert()
        .success();
    let gone = expected_worktree(repo.path(), "gone");
    std::fs::remove_dir_all(&gone).unwrap();

    let output = cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .arg("list")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("gone  prunable"), "{stdout}");
}

#[test]
fn remove_refuses_dirty_and_then_removes_clean_worktree_but_keeps_branch() {
    let repo = repo();
    cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["switch", "--create", "topic"])
        .assert()
        .success();
    let topic = expected_worktree(repo.path(), "topic");
    std::fs::write(Path::new(&topic).join("dirty.txt"), "dirty").unwrap();
    cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["remove", "topic"])
        .assert()
        .failure();
    std::fs::remove_file(Path::new(&topic).join("dirty.txt")).unwrap();
    cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["remove", "topic"])
        .assert()
        .success();
    assert!(!Path::new(&topic).exists());
    assert_eq!(
        git(
            repo.path(),
            &["show-ref", "--verify", "--hash", "refs/heads/topic"]
        ),
        git(repo.path(), &["rev-parse", "main"])
    );
}

#[test]
fn remove_current_worktree_prints_primary_path_for_shell_wrapper() {
    let repo = repo();
    cargo_bin_cmd!("grove")
        .current_dir(repo.path())
        .args(["switch", "--create", "topic"])
        .assert()
        .success();
    let topic = expected_worktree(repo.path(), "topic");
    let output = cargo_bin_cmd!("grove")
        .current_dir(&topic)
        .arg("remove")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        repo.path().canonicalize().unwrap().to_str().unwrap()
    );
}

#[test]
fn shell_zsh_wraps_switch_with_cd() {
    let output = cargo_bin_cmd!("grove")
        .args(["shell", "zsh"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("grove()"));
    assert!(stdout.contains("builtin cd"));
    assert!(stdout.contains("command grove"));
}
