use std::{
    ffi::OsStr,
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use tempfile::TempDir;

pub struct TestRepo {
    _root: TempDir,
    repo: PathBuf,
    home: PathBuf,
    git_config: PathBuf,
    navigation: PathBuf,
    agent_log: PathBuf,
    agent_pid: PathBuf,
    agent: PathBuf,
    runtime_socket: PathBuf,
}

pub struct TestChange {
    pub branch: String,
    pub path: PathBuf,
}

struct PtyProcess {
    child: Child,
    output: tempfile::NamedTempFile,
}

impl TestRepo {
    pub fn new() -> Self {
        let root = tempfile::tempdir().expect("create test directory");
        let repo = root.path().join("repo");
        let home = root.path().join("home");
        let git_config = root.path().join("gitconfig");
        let navigation = root.path().join("navigation");
        let agent_log = root.path().join("agent.log");
        let agent_pid = root.path().join("agent.pid");
        let agent = root.path().join("agent");
        let runtime_socket = root.path().join("runtime/rmux.sock");
        fs::create_dir(&home).expect("create test home");

        let fixture = Self {
            _root: root,
            repo,
            home,
            git_config,
            navigation,
            agent_log,
            agent_pid,
            agent,
            runtime_socket,
        };
        fixture.git_from(
            fixture._root.path(),
            ["config", "--global", "user.name", "Grove Test"],
        );
        fixture.git_from(
            fixture._root.path(),
            ["config", "--global", "user.email", "grove@example.test"],
        );
        fixture.initialize(&fixture.repo);
        fixture.configure_agent();
        fixture
    }

    pub fn create_repo(&self, relative: impl AsRef<Path>) -> PathBuf {
        let repo = self._root.path().join(relative);
        self.initialize(&repo);
        repo
    }

    pub fn path(&self) -> &Path {
        &self.repo
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn worktree(&self, branch: &str) -> PathBuf {
        self.git(["worktree", "list", "--porcelain"])
            .split("\n\n")
            .find_map(|record| {
                let matches = record
                    .lines()
                    .any(|line| line == format!("branch refs/heads/{branch}"));
                matches.then(|| {
                    PathBuf::from(
                        record
                            .lines()
                            .find_map(|line| line.strip_prefix("worktree "))
                            .expect("worktree record has a path"),
                    )
                })
            })
            .unwrap_or_else(|| panic!("missing worktree for branch {branch}"))
    }

    pub fn grove(&self) -> assert_cmd::Command {
        self.grove_from(&self.repo)
    }

    pub fn create_change(&self, task: &str, from: Option<&str>) -> TestChange {
        self.create_change_from(&self.repo, task, from)
    }

    pub fn create_change_from(
        &self,
        directory: &Path,
        task: &str,
        from: Option<&str>,
    ) -> TestChange {
        let branch = task
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let mut command = self.grove_from(directory);
        command.args(["new", "--shell"]);
        if let Some(from) = from {
            command.args(["--from", from]);
        }
        command.arg(&branch).assert().success();
        let path = self.navigation();
        let branch = self.git_from(&path, ["branch", "--show-current"]);
        TestChange { branch, path }
    }

    pub fn grove_from(&self, directory: &Path) -> assert_cmd::Command {
        let mut command = assert_cmd::Command::cargo_bin("grove").expect("compiled grove binary");
        command
            .current_dir(directory)
            .env("HOME", &self.home)
            .env_remove("XDG_CONFIG_HOME")
            .env("GIT_CONFIG_GLOBAL", &self.git_config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GROVE_DIRECTIVE_CD_FILE", &self.navigation)
            .env("GROVE_TEST_AGENT_LOG", &self.agent_log);
        command.env("GROVE_TEST_AGENT_PID", &self.agent_pid);
        command.env("PATH", self.test_path());
        command.env("XDG_RUNTIME_DIR", &self.runtime_socket);
        command
    }

    pub fn switch_in_pty(&self, ready: &str, input: &[u8]) -> Output {
        let binary = assert_cmd::Command::cargo_bin("grove")
            .expect("compiled grove binary")
            .get_program()
            .to_owned();
        let mut command = self.pty(&self.repo, OsStr::new("/bin/sh"));
        command
            .args([
                "-c",
                "\"$GROVE_TEST_BINARY\" switch --shell\nstatus=$?\nstty -a\nexit \"$status\"",
            ])
            .env("GROVE_TEST_BINARY", binary);
        let mut picker = PtyProcess::start(&mut command, self._root.path());
        picker.wait_for(ready, Duration::from_secs(10), "Grove switch");
        picker.send(input, "Grove switch");
        let status = picker.wait_for_exit(Duration::from_secs(5), "Grove switch");
        Output {
            status,
            stdout: picker.output(),
            stderr: Vec::new(),
        }
    }

    pub fn detach_picked_switch(&self, ready: &str, input: &[u8]) {
        let mut command = self.grove_pty(&self.repo);
        command.arg("switch");
        let mut picker = PtyProcess::start(&mut command, self._root.path());
        picker.wait_for(ready, Duration::from_secs(10), "Grove switch");
        picker.send(input, "Grove switch");
        picker.wait_ready();
        picker.detach();
    }

    pub fn select_agent_in_pty(&self, ready: &str, input: &[u8]) -> Output {
        let mut command = self.grove_pty(&self.repo);
        command.arg("switch");
        let mut picker = PtyProcess::start(&mut command, self._root.path());
        picker.wait_for(ready, Duration::from_secs(10), "Grove switch");
        picker.send(input, "Grove switch");
        let status = picker.wait_for_exit(Duration::from_secs(5), "Grove switch");
        Output {
            status,
            stdout: picker.output(),
            stderr: Vec::new(),
        }
    }

    pub fn agent_log(&self) -> String {
        fs::read_to_string(&self.agent_log).unwrap_or_default()
    }

    pub fn agent_pids(&self) -> Vec<u32> {
        fs::read_to_string(&self.agent_pid)
            .unwrap_or_default()
            .lines()
            .map(|pid| pid.parse().expect("agent PID is an integer"))
            .collect()
    }

    pub fn process_running(&self, pid: u32) -> bool {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    pub fn wait_for_process_exit(&self, pid: u32) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.process_running(pid) {
            assert!(Instant::now() < deadline, "agent process did not exit");
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn stop_process(&self, pid: u32) {
        let status = Command::new("kill")
            .arg(pid.to_string())
            .status()
            .expect("stop agent process");
        assert!(status.success(), "could not stop agent process {pid}");
        self.wait_for_process_exit(pid);
    }

    pub fn runtime_exists(&self) -> bool {
        self.runtime_socket.exists()
    }

    pub fn pi_session_files(&self) -> Vec<PathBuf> {
        let sessions = self.home.join(".local/state/grove/sessions");
        let Ok(entries) = fs::read_dir(sessions) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect()
    }

    fn grove_pty(&self, directory: &Path) -> Command {
        let binary = assert_cmd::Command::cargo_bin("grove")
            .expect("compiled grove binary")
            .get_program()
            .to_owned();
        self.pty(directory, &binary)
    }

    fn pty(&self, directory: &Path, program: &OsStr) -> Command {
        let mut command = Command::new("script");
        command
            .args([OsStr::new("-q"), OsStr::new("/dev/null")])
            .arg(program)
            .current_dir(directory)
            .env("HOME", &self.home)
            .env_remove("XDG_CONFIG_HOME")
            .env("GIT_CONFIG_GLOBAL", &self.git_config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GROVE_DIRECTIVE_CD_FILE", &self.navigation)
            .env("GROVE_TEST_AGENT_LOG", &self.agent_log)
            .env("GROVE_TEST_AGENT_PID", &self.agent_pid)
            .env("PATH", self.test_path())
            .env("XDG_RUNTIME_DIR", &self.runtime_socket);
        command
    }

    pub fn detach_new(&self, branch: &str) {
        let mut command = self.grove_pty(&self.repo);
        command.args(["new", branch]);
        let mut agent = PtyProcess::start(&mut command, self._root.path());
        agent.wait_ready();
        agent.detach();
    }

    pub fn detach_switch(&self, branch: &str) {
        let mut command = self.grove_pty(&self.repo);
        command.args(["switch", branch]);
        let mut agent = PtyProcess::start(&mut command, self._root.path());
        agent.wait_ready();
        agent.detach();
    }

    pub fn detach_inferred_new(&self, prompt: &str) -> PathBuf {
        let mut command = self.grove_pty(&self.repo);
        command.arg("new");
        let mut agent = PtyProcess::start(&mut command, self._root.path());
        agent.wait_ready();
        let pending = self
            .agent_log()
            .lines()
            .find_map(|line| line.strip_prefix("cwd="))
            .map(PathBuf::from)
            .expect("agent logged its worktree");
        agent.send(format!("{prompt}\n").as_bytes(), "Grove agent prompt");
        agent.wait_for(
            "grove-test-prompt-received",
            Duration::from_secs(5),
            "Grove agent prompt",
        );
        agent.detach();
        let branch = prompt
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.branch_exists(&branch)
            || self.worktree(&branch).file_name() != Some(OsStr::new(&branch))
        {
            assert!(
                Instant::now() < deadline,
                "Grove did not infer a branch from the first prompt"
            );
            thread::sleep(Duration::from_millis(20));
        }
        let worktree = self.worktree(&branch);
        assert!(!pending.exists(), "pending directory was not renamed");
        worktree
    }

    pub fn detach_unnamed_new_without_prompt(&self) -> (Output, PathBuf) {
        let mut command = self.grove_pty(&self.repo);
        command.arg("new");
        let mut agent = PtyProcess::start(&mut command, self._root.path());
        agent.wait_ready();
        let worktree = self
            .agent_log()
            .lines()
            .find_map(|line| line.strip_prefix("cwd="))
            .map(PathBuf::from)
            .expect("agent logged its worktree");
        agent.send(b"\x1c", "Grove agent");
        let status = agent.wait_for_exit(Duration::from_secs(5), "Grove agent");
        (
            Output {
                status,
                stdout: agent.output(),
                stderr: Vec::new(),
            },
            worktree,
        )
    }

    pub fn exit_unnamed_new_without_prompt(&self) -> Output {
        let mut command = self.grove_pty(&self.repo);
        command.arg("new");
        let mut agent = PtyProcess::start(&mut command, self._root.path());
        agent.wait_ready();
        agent.send(b"\x04", "Grove agent");
        let status = agent.wait_for_exit(Duration::from_secs(5), "Grove agent");
        Output {
            status,
            stdout: agent.output(),
            stderr: Vec::new(),
        }
    }

    pub fn detach_dirty_unnamed_new_without_prompt(&self) -> (Output, PathBuf) {
        let mut command = self.grove_pty(&self.repo);
        command.arg("new");
        let mut agent = PtyProcess::start(&mut command, self._root.path());
        agent.wait_ready();
        let worktree = self
            .agent_log()
            .lines()
            .find_map(|line| line.strip_prefix("cwd="))
            .map(PathBuf::from)
            .expect("agent logged its worktree");
        fs::write(worktree.join("agent-created.txt"), "keep me")
            .expect("write an agent-created file");
        agent.send(b"\x1c", "Grove agent");
        let status = agent.wait_for_exit(Duration::from_secs(5), "Grove agent");
        (
            Output {
                status,
                stdout: agent.output(),
                stderr: Vec::new(),
            },
            worktree,
        )
    }
}

impl PtyProcess {
    fn start(command: &mut Command, output_directory: &Path) -> Self {
        let output = tempfile::NamedTempFile::new_in(output_directory).expect("create PTY output");
        let child = command
            .stdin(Stdio::piped())
            .stdout(output.reopen().expect("open PTY output"))
            .stderr(output.reopen().expect("open PTY errors"))
            .spawn()
            .expect("start command in a PTY");
        Self { child, output }
    }

    fn wait_for(&mut self, expected: &str, timeout: Duration, label: &str) {
        let deadline = Instant::now() + timeout;
        loop {
            let captured = self.output();
            let output = String::from_utf8_lossy(&captured);
            if output.contains(expected) {
                break;
            }
            if let Some(status) = self.child.try_wait().expect("inspect PTY command") {
                panic!("{label} exited before it was ready: {status}\n{output}");
            }
            if Instant::now() >= deadline {
                self.stop();
                panic!("{label} did not become ready before timeout\n{output}");
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn send(&mut self, input: &[u8], label: &str) {
        self.child
            .stdin
            .as_mut()
            .unwrap_or_else(|| panic!("{label} stdin is unavailable"))
            .write_all(input)
            .unwrap_or_else(|error| panic!("send input to {label}: {error}"));
    }

    fn wait_for_exit(&mut self, timeout: Duration, label: &str) -> ExitStatus {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().expect("inspect PTY command") {
                return status;
            }
            if Instant::now() >= deadline {
                self.stop();
                panic!(
                    "{label} did not exit before timeout\n{}",
                    String::from_utf8_lossy(&self.output())
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn output(&self) -> Vec<u8> {
        fs::read(self.output.path()).expect("read PTY output")
    }

    fn stop(&mut self) {
        self.child.kill().expect("kill PTY command");
        self.child.wait().expect("reap PTY command");
    }

    fn wait_ready(&mut self) {
        self.wait_for(
            "grove-test-agent-ready",
            Duration::from_secs(10),
            "Grove agent",
        );
    }

    fn detach(&mut self) {
        self.send(b"\x1c", "Grove agent");
        let status = self.wait_for_exit(Duration::from_secs(5), "Grove agent");
        assert!(status.success(), "Grove agent detach failed: {status}");
    }
}

impl Drop for PtyProcess {
    fn drop(&mut self) {
        if self.child.try_wait().is_ok_and(|status| status.is_none()) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

impl TestRepo {
    pub fn remove_pi(&self) {
        fs::remove_file(self.home.join("bin/pi")).expect("remove fake Pi executable");
    }

    pub fn git<I, S>(&self, args: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.git_from(&self.repo, args)
    }

    pub fn git_from<I, S>(&self, directory: &Path, args: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let (args, output) = self.git_output(directory, args);
        assert_git_success(directory, &args, &output);
        String::from_utf8(output.stdout)
            .expect("Git stdout is UTF-8")
            .trim()
            .to_owned()
    }

    pub fn git_optional<I, S>(&self, args: I) -> Option<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let (_, output) = self.git_output(&self.repo, args);
        if output.status.success() {
            Some(
                String::from_utf8(output.stdout)
                    .expect("Git stdout is UTF-8")
                    .trim()
                    .to_owned(),
            )
        } else {
            None
        }
    }

    pub fn commit_file(&self, directory: &Path, relative: &str, contents: &str) -> String {
        fs::write(directory.join(relative), contents).expect("write committed file");
        self.git_from(directory, ["add", relative]);
        self.git_from(directory, ["commit", "-m", &format!("Add {relative}")]);
        self.git_from(directory, ["rev-parse", "HEAD"])
    }

    pub fn branch_exists(&self, branch: &str) -> bool {
        self.git_optional(["rev-parse", "--verify", &format!("refs/heads/{branch}")])
            .is_some()
    }

    pub fn config(&self, key: &str) -> Option<String> {
        self.git_optional(["config", "--local", "--get", key])
    }

    pub fn has_lineage(&self, branch: &str) -> bool {
        self.git_optional([
            "config",
            "--local",
            "--get-regexp",
            &format!("^branch\\.{branch}\\.grove-"),
        ])
        .is_some()
    }

    pub fn navigation(&self) -> PathBuf {
        let value = fs::read_to_string(&self.navigation).expect("read Grove navigation directive");
        PathBuf::from(value)
    }

    fn configure(&self, command: &mut Command) {
        command
            .env("HOME", &self.home)
            .env("GIT_CONFIG_GLOBAL", &self.git_config)
            .env("GIT_CONFIG_NOSYSTEM", "1");
    }

    fn git_output<I, S>(&self, directory: &Path, args: I) -> (Vec<std::ffi::OsString>, Output)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_owned())
            .collect::<Vec<_>>();
        let mut command = Command::new("git");
        self.configure(&mut command);
        let output = command
            .current_dir(directory)
            .args(&args)
            .output()
            .unwrap_or_else(|error| {
                panic!(
                    "failed to run git in {} with {args:?}: {error}",
                    directory.display()
                )
            });
        (args, output)
    }

    fn initialize(&self, repo: &Path) {
        self.git_from(
            self._root.path(),
            [
                OsStr::new("init"),
                OsStr::new("--initial-branch=main"),
                repo.as_os_str(),
            ],
        );
        fs::write(repo.join("README.md"), "# Test repository\n").expect("write initial file");
        self.git_from(repo, ["add", "README.md"]);
        self.git_from(repo, ["commit", "-m", "Initial commit"]);
    }

    fn configure_agent(&self) {
        fs::write(&self.agent, include_str!("agent.sh")).expect("write test agent");
        fs::set_permissions(&self.agent, fs::Permissions::from_mode(0o755))
            .expect("make test agent executable");
        let bin = self.home.join("bin");
        fs::create_dir(&bin).expect("create test bin directory");
        fs::copy(&self.agent, bin.join("pi")).expect("install fake Pi executable");
    }

    fn test_path(&self) -> std::ffi::OsString {
        let paths = vec![
            self.home.join("bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ];
        std::env::join_paths(paths).expect("build test PATH")
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        for pid in self.agent_pids() {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
    }
}

fn assert_git_success(directory: &Path, args: &[std::ffi::OsString], output: &Output) {
    assert!(
        output.status.success(),
        "git command failed\n  cwd: {}\n  args: {args:?}\n  status: {}\n  stdout: {}\n  stderr: {}",
        directory.display(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
