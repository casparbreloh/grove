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
    pub id: String,
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
        let mut command = self.grove_from(directory);
        command.args(["switch", "--create"]);
        if let Some(from) = from {
            command.args(["--from", from]);
        }
        command.arg(task).assert().success();
        let path = self.navigation();
        let id = self.git_from(&path, ["branch", "--show-current"]);
        TestChange { id, path }
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
        command.env("GROVE_RUNTIME_SOCKET", &self.runtime_socket);
        command
    }

    pub fn switch_in_pty(&self, ready: &str, input: &[u8]) -> Output {
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

    pub fn runtime_exists(&self) -> bool {
        self.runtime_socket.exists()
    }

    fn grove_pty(&self, directory: &Path) -> Command {
        let binary = assert_cmd::Command::cargo_bin("grove")
            .expect("compiled grove binary")
            .get_program()
            .to_owned();
        let mut command = Command::new("script");
        command
            .args([OsStr::new("-q"), OsStr::new("/dev/null")])
            .arg(binary)
            .current_dir(directory)
            .env("HOME", &self.home)
            .env_remove("XDG_CONFIG_HOME")
            .env("GIT_CONFIG_GLOBAL", &self.git_config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GROVE_DIRECTIVE_CD_FILE", &self.navigation)
            .env("GROVE_TEST_AGENT_LOG", &self.agent_log)
            .env("GROVE_TEST_AGENT_PID", &self.agent_pid)
            .env("PATH", self.test_path())
            .env("GROVE_RUNTIME_SOCKET", &self.runtime_socket);
        command
    }

    fn agent_pty(&self, directory: &Path, name: Option<&str>) -> Command {
        let mut command = self.grove_pty(directory);
        command.arg("agent");
        if let Some(name) = name {
            command.arg(name);
        }
        command
    }

    pub fn detach_agent(&self, directory: &Path, name: Option<&str>) -> String {
        let mut agent = self.start_agent(directory, name);
        agent.wait_ready();
        let terminal = fs::read_to_string(agent.output.path()).expect("read agent PTY output");
        agent.detach();
        terminal
    }

    pub fn detach_agents_concurrently(&self, directory: &Path, count: usize) {
        let mut agents = (0..count)
            .map(|_| self.start_agent(directory, None))
            .collect::<Vec<_>>();
        for agent in &mut agents {
            agent.wait_ready();
        }
        for agent in &mut agents {
            agent.detach();
        }
    }

    fn start_agent(&self, directory: &Path, name: Option<&str>) -> PtyProcess {
        let mut command = self.agent_pty(directory, name);
        PtyProcess::start(&mut command, self._root.path())
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
            .take()
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
        self.send(b"\x02d", "Grove agent");
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
    pub fn use_prompt_template(&self) {
        fs::write(
            self.home.join(".config/grove/grove.toml"),
            format!(
                "agent = \"test\"\n\n[agents.test]\ncommand = [\"{}\", \"--prompt={{prompt}}\"]\n",
                self.agent.display()
            ),
        )
        .expect("write stale agent config");
    }

    pub fn select_project_agent(&self, directory: &Path, name: &str) {
        fs::write(
            directory.join("grove.toml"),
            format!("agent = \"{name}\"\n"),
        )
        .expect("write project Grove config");
    }

    pub fn use_missing_agent_command(&self) {
        fs::write(
            self.home.join(".config/grove/grove.toml"),
            "agent = \"missing\"\n\n[agents.missing]\ncommand = [\"/grove-test/missing-agent\"]\n",
        )
        .expect("write missing agent config");
    }

    pub fn use_builtin_defaults(&self) {
        fs::remove_file(self.home.join(".config/grove/grove.toml"))
            .expect("remove global Grove config");
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
        fs::write(
            &self.agent,
            "#!/bin/sh\nprintf '%s\\n' \"$$\" >> \"$GROVE_TEST_AGENT_PID\"\nprintf 'cwd=%s\\ndirective=%s\\n' \"$PWD\" \"${GROVE_DIRECTIVE_CD_FILE-absent}\" >> \"$GROVE_TEST_AGENT_LOG\"\nfor argument do printf 'arg=<%s>\\n' \"$argument\" >> \"$GROVE_TEST_AGENT_LOG\"; done\nprintf 'grove-test-agent-ready\\n'\nsleep 30\n",
        )
        .expect("write test agent");
        fs::set_permissions(&self.agent, fs::Permissions::from_mode(0o755))
            .expect("make test agent executable");
        let bin = self.home.join("bin");
        fs::create_dir(&bin).expect("create test bin directory");
        fs::copy(&self.agent, bin.join("pi")).expect("install fake Pi executable");
        let config_dir = self.home.join(".config/grove");
        fs::create_dir_all(&config_dir).expect("create global Grove config directory");
        fs::write(
            config_dir.join("grove.toml"),
            format!(
                "agent = \"test\"\n\n[agents.test]\ncommand = [\"{}\", \"session\", \"space value\", \"quote'\\\"\", \"\"]\n\n[agents.project]\ncommand = [\"{}\", \"project-session\"]\n",
                self.agent.display(),
                self.agent.display()
            ),
        )
        .expect("write global Grove config");
    }

    fn test_path(&self) -> std::ffi::OsString {
        let mut paths = vec![self.home.join("bin")];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        std::env::join_paths(paths).expect("build test PATH")
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        if let Ok(mut connection) = rmux_client::connect(&self.runtime_socket) {
            let _ = connection.kill_server();
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
