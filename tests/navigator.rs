mod support;

use std::fs;

use support::{
    TestRepo, assert_inline_terminal_restored, assert_terminal_restored, stderr, stdout,
};

#[test]
fn command_and_shell_navigation_is_one_coherent_workflow() {
    let repo = TestRepo::new();
    let help = stdout(repo.grove().arg("--help").assert().success().get_output());
    for command in ["new", "sync", "ship", "archive", "init"] {
        assert!(help.contains(command), "{help}");
    }
    for (command, usage, flag) in [
        ("new", "Usage: grove new [OPTIONS]", "--from <REF>"),
        ("archive", "Usage: grove archive [OPTIONS]", "--force"),
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
        assert!(script.contains("GROVE_EXECUTABLE"), "{script}");
        assert!(script.contains("COMPLETE"), "{script}");

        let shell_repo = TestRepo::new();
        fs::create_dir_all(shell_repo.path().join("matching/subdirectory")).unwrap();
        shell_repo.commit_file(
            shell_repo.path(),
            "matching/subdirectory/tracked.txt",
            "tracked\n",
        );
        let change = shell_repo.create_change(None);
        shell_repo.set_change_title(&change, "Navigate With Shell");

        let before_pi = shell_repo.agent_log().matches("mode=interactive").count();
        let output = shell_repo.navigator_from_shell_in_pty(
            &shell_repo.path().join("matching/subdirectory"),
            shell,
            "Navigate With Shell",
            b"nAvIgAtE\x1b[B\t",
        );
        assert!(output.status.success(), "{shell}: {output:?}");
        let terminal = stdout(&output);
        assert!(
            terminal.contains(&format!(
                "__PWD__{}",
                change
                    .path
                    .join("matching/subdirectory")
                    .canonicalize()
                    .unwrap()
                    .display()
            )),
            "{shell}: {terminal}"
        );
        assert_eq!(
            shell_repo.agent_log().matches("mode=interactive").count(),
            before_pi,
            "Tab must not open Pi"
        );
        assert_terminal_restored(&terminal);

        let output = shell_repo.navigator_from_shell_in_pty(
            &change.path.join("matching/subdirectory"),
            shell,
            "Navigate With Shell",
            b"\x1b[A\r",
        );
        assert!(output.status.success(), "{shell}: {output:?}");
        let terminal = stdout(&output);
        assert!(
            terminal.contains(&format!(
                "__PWD__{}",
                shell_repo
                    .path()
                    .join("matching/subdirectory")
                    .canonicalize()
                    .unwrap()
                    .display()
            )),
            "{shell}: {terminal}"
        );
        assert_terminal_restored(&terminal);

        fs::create_dir(shell_repo.path().join("main-only")).unwrap();
        let output = shell_repo.navigator_from_shell_in_pty(
            &shell_repo.path().join("main-only"),
            shell,
            "Navigate With Shell",
            b"\x1b[B\t",
        );
        assert!(output.status.success(), "{shell}: {output:?}");
        let terminal = stdout(&output);
        assert!(
            terminal.contains(&format!(
                "__PWD__{}",
                change.path.canonicalize().unwrap().display()
            )),
            "{shell}: {terminal}"
        );
    }

    let change = repo.create_change(None);
    let output = repo
        .grove_from(&change.path)
        .env_remove("GROVE_DIRECTIVE_CD_FILE")
        .arg("archive")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&output).contains("shell integration is not loaded"));
    assert!(change.path.exists());

    let invalid_archive = TestRepo::new();
    let change = invalid_archive.create_change(None);
    let output = invalid_archive
        .grove_from(&change.path)
        .env("GROVE_DIRECTIVE_CD_FILE", invalid_archive.path())
        .arg("archive")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&output).contains("shell navigation directive"));
    assert!(change.path.exists());

    let commands = stdout(
        repo.grove()
            .env("COMPLETE", "fish")
            .args(["--", "grove", ""])
            .assert()
            .success()
            .get_output(),
    );
    assert!(commands.contains("archive\t"), "{commands}");
    let flags = stdout(
        repo.grove()
            .env("COMPLETE", "fish")
            .args(["--", "grove", "new", "--"])
            .assert()
            .success()
            .get_output(),
    );
    assert!(flags.contains("--from"), "{flags}");
}

#[test]
fn bare_navigator_restores_the_plain_picker_and_dispatches_rows() {
    let repo = TestRepo::new();
    let mut changes = [
        repo.create_change(None),
        repo.create_change(None),
        repo.create_change(None),
        repo.create_change(None),
        repo.create_change(None),
    ];
    changes.sort_by_key(|change| {
        (
            repo.change_record(change.path.parent().unwrap())["created_at"]
                .as_u64()
                .unwrap(),
            change.id.clone(),
        )
    });
    repo.set_change_title(&changes[0], "Capture Native Sessions");
    repo.set_change_title(&changes[1], "Capture Native Sessions");
    repo.set_change_title(&changes[2], "Deploy API");
    repo.set_change_title(&changes[4], "New Change");
    fs::write(changes[2].path.join("uncommitted.txt"), "change\n").unwrap();
    repo.commit_file(repo.path(), "main-advance.txt", "main advance\n");

    let ordinary = repo.home().join("ordinary");
    repo.git(["branch", "ordinary"]);
    repo.git(["worktree", "add", ordinary.to_str().unwrap(), "ordinary"]);
    let detached = repo.home().join("detached");
    repo.git(["worktree", "add", "--detach", detached.to_str().unwrap()]);

    let unstyled = repo.navigator_without_color_in_pty(repo.path(), "New Change", b"\x1b");
    assert!(unstyled.status.success(), "{unstyled:?}");
    let terminal = stdout(&unstyled);
    let main = terminal.find("Main").expect("Main row");
    let first = terminal
        .find("Capture Native Sessions")
        .expect("first Change row");
    let deploy = terminal.find("Deploy API").expect("Change row");
    let untitled = terminal.find("Untitled").expect("untitled Change row");
    let titled_new = terminal.find("New Change").expect("New Change title");
    assert!(
        main < first && first < deploy && deploy < untitled && untitled < titled_new,
        "{terminal}"
    );
    assert_eq!(terminal.matches("New Change").count(), 1, "{terminal}");
    for change in &changes {
        assert!(terminal.contains(&change.id), "{terminal}");
    }
    for metadata in ["Base", "Changes", "Base↕", "Path"] {
        assert!(
            terminal.contains(metadata),
            "missing {metadata}: {terminal}"
        );
    }
    assert!(!terminal.contains("ordinary") && !terminal.contains("detached"));
    assert!(!terminal.contains("Filter:"), "{terminal}");
    assert!(terminal.contains("› Main"), "{terminal}");
    assert_no_navigator_legend(&terminal);
    assert_no_sgr(&terminal);
    assert_inline_terminal_restored(&terminal);

    let before_pi = repo.agent_log().matches("mode=interactive").count();
    let selected =
        repo.navigator_in_pty(repo.path(), "New Change", b"ignored text\x7f\x1b[B\x1b[B\r");
    assert!(selected.status.success(), "{selected:?}");
    let terminal = stdout(&selected);
    assert_no_navigator_legend(&terminal);
    assert!(
        terminal.contains("\x1b[1m  Title"),
        "header is not bold: {terminal:?}"
    );
    assert!(
        !terminal.contains("\x1b[1mCapture Native Sessions"),
        "selected row became bold: {terminal:?}"
    );
    assert!(!terminal.contains("\x1b[2m"), "{terminal:?}");
    assert_no_foreground_colors(&terminal);
    assert_eq!(
        repo.agent_log().matches("mode=interactive").count(),
        before_pi + 1
    );
    let log = repo.agent_log();
    let last_launch = log.rsplit("mode=interactive\n").next().unwrap();
    assert!(
        last_launch.contains(&format!(
            "cwd={}",
            changes[1].path.canonicalize().unwrap().display()
        )),
        "{last_launch}"
    );
    assert_inline_terminal_restored(&terminal);

    let before_pi = repo.agent_log().matches("mode=interactive").count();
    let workspace = repo.navigator_in_pty(repo.path(), "New Change", b"\x1b[B\x1b[B\x1b[B\t");
    assert!(workspace.status.success(), "{workspace:?}");
    assert_eq!(repo.navigation(), changes[2].path.canonicalize().unwrap());
    assert_eq!(
        repo.agent_log().matches("mode=interactive").count(),
        before_pi,
        "Tab must only navigate"
    );

    for input in [b"\x1b[A\r".as_slice(), b"\x1b[A\t".as_slice()] {
        let before_pi = repo.agent_log().matches("mode=interactive").count();
        let main = repo.navigator_without_color_in_pty(repo.path(), "New Change", input);
        assert!(main.status.success(), "{main:?}");
        assert_eq!(repo.navigation(), repo.path().canonicalize().unwrap());
        assert_eq!(
            repo.agent_log().matches("mode=interactive").count(),
            before_pi,
            "Main must never launch Pi"
        );
        assert_inline_terminal_restored(&stdout(&main));
    }

    let before = repo.navigation();
    let unmanaged = repo.navigator_in_pty(&ordinary, "New Change", b"\x1b");
    assert!(unmanaged.status.success(), "{unmanaged:?}");
    assert!(!stdout(&unmanaged).contains("ordinary"));
    assert_eq!(repo.navigation(), before);
    assert_inline_terminal_restored(&stdout(&unmanaged));
    for input in [b"\x1b".as_slice(), b"\x03".as_slice()] {
        let cancelled = repo.navigator_in_pty(repo.path(), "New Change", input);
        assert!(cancelled.status.success(), "{cancelled:?}");
        assert_eq!(repo.navigation(), before);
        assert_inline_terminal_restored(&stdout(&cancelled));
    }

    let non_tty = repo.grove().assert().failure().get_output().clone();
    assert!(stderr(&non_tty).contains("interactive Change navigation requires a terminal"));
    let non_tty_archive = repo
        .grove()
        .arg("archive")
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&non_tty_archive).contains("interactive Change selection requires a terminal"));

    let corrupt = TestRepo::new();
    let change = corrupt.create_change(None);
    fs::write(
        change.path.parent().unwrap().join("change.json"),
        "not json\n",
    )
    .unwrap();
    let error = corrupt.grove().assert().failure().get_output().clone();
    assert!(stderr(&error).contains("invalid change record"));
    assert!(change.path.exists());
}

#[test]
fn navigator_rows_adapt_to_narrow_terminals_without_losing_change_ids() {
    let repo = TestRepo::new();
    let first = repo.create_change(None);
    let second = repo.create_change(None);
    repo.set_change_title(&first, "A Very Long Duplicate Narrow Picker Change Title");
    repo.set_change_title(&second, "A Very Long Duplicate Narrow Picker Change Title");
    let full_path = format!(
        "~/{}",
        first.path.strip_prefix(repo.home()).unwrap().display()
    );

    let navigated = repo.navigator_in_narrow_pty("A Very Long", b"\x1b[B\x1b");
    assert!(navigated.status.success(), "{navigated:?}");
    let terminal = stdout(&navigated);
    assert!(!terminal.contains(&full_path), "{terminal}");
    assert!(terminal.contains(&first.id), "{terminal}");
    assert!(terminal.contains(&second.id), "{terminal}");
    assert!(first.path.exists() && second.path.exists());
    assert_inline_terminal_restored(&terminal);

    let short = repo.navigator_in_short_pty();
    assert!(short.status.success(), "{short:?}");
    assert!(stdout(&short).contains("Main"), "{}", stdout(&short));
    assert_inline_terminal_restored(&stdout(&short));
}

fn assert_no_navigator_legend(terminal: &str) {
    for text in [
        "↑↓",
        "enter shell",
        "enter agent",
        "tab shell",
        "esc close",
        "esc/ctrl-c",
    ] {
        assert!(
            !terminal.contains(text),
            "navigator rendered {text:?}: {terminal}"
        );
    }
}

fn sgr_parameters(terminal: &str) -> impl Iterator<Item = &str> {
    terminal.split("\x1b[").skip(1).filter_map(|escape| {
        let end = escape.find(|character: char| !character.is_ascii_digit() && character != ';')?;
        (escape.as_bytes()[end] == b'm').then_some(&escape[..end])
    })
}

fn assert_no_sgr(terminal: &str) {
    assert!(
        sgr_parameters(terminal).next().is_none(),
        "styling was not disabled: {terminal:?}"
    );
}

fn assert_no_foreground_colors(terminal: &str) {
    for parameters in sgr_parameters(terminal) {
        for parameter in parameters
            .split(';')
            .filter_map(|value| value.parse::<u8>().ok())
        {
            assert!(
                !matches!(parameter, 30..=39 | 90..=97),
                "titles must not be globally colored: {terminal:?}"
            );
        }
    }
}
