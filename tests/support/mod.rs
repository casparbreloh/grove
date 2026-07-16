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
    agent: PathBuf,
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
        let agent = root.path().join("agent");
        fs::create_dir(&home).expect("create test home");

        let fixture = Self {
            _root: root,
            repo,
            home,
            git_config,
            navigation,
            agent_log,
            agent,
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

    pub fn change_capsules(&self) -> Vec<PathBuf> {
        let root = self.home.join(".grove");
        let Ok(repositories) = fs::read_dir(root) else {
            return Vec::new();
        };
        let mut capsules = repositories
            .filter_map(Result::ok)
            .flat_map(|repository| {
                fs::read_dir(repository.path())
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .collect::<Vec<_>>()
            })
            .filter(|path| path.join("change.json").is_file())
            .collect::<Vec<_>>();
        capsules.sort();
        capsules
    }

    pub fn grove(&self) -> assert_cmd::Command {
        self.grove_from(&self.repo)
    }

    pub fn create_change(&self, from: Option<&str>) -> TestChange {
        self.create_change_from(&self.repo, from)
    }

    pub fn set_change_title(&self, change: &TestChange, title: &str) {
        let record_path = change
            .path
            .parent()
            .expect("change capsule")
            .join("change.json");
        let mut record: serde_json::Value =
            serde_json::from_slice(&fs::read(&record_path).expect("read change record"))
                .expect("valid change record");
        record["title"] = title.into();
        fs::write(
            record_path,
            serde_json::to_vec_pretty(&record).expect("serialize change record"),
        )
        .expect("write change record");
    }

    pub fn create_change_from(&self, directory: &Path, from: Option<&str>) -> TestChange {
        let mut command = self.grove_from(directory);
        command.args(["new", "--shell"]);
        if let Some(from) = from {
            command.args(["--from", from]);
        }
        command.assert().success();
        let path = self.navigation();
        let branch = self.git_from(&path, ["branch", "--show-current"]);
        TestChange { branch, path }
    }

    pub fn grove_from(&self, directory: &Path) -> assert_cmd::Command {
        assert_cmd::Command::from_std(self.compiled_grove(directory))
    }

    pub fn spawn_grove_from<const N: usize>(&self, directory: &Path, args: [&str; N]) -> Child {
        self.compiled_grove(directory)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn compiled Grove binary")
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

    pub fn remove_in_pty(&self, ready: &str, input: &[u8]) -> Output {
        let binary = assert_cmd::Command::cargo_bin("grove")
            .expect("compiled grove binary")
            .get_program()
            .to_owned();
        let mut command = self.pty(&self.repo, OsStr::new("/bin/sh"));
        command
            .args([
                "-c",
                "\"$GROVE_TEST_BINARY\" remove\nstatus=$?\nstty -a\nexit \"$status\"",
            ])
            .env("GROVE_TEST_BINARY", binary);
        let mut picker = PtyProcess::start(&mut command, self._root.path());
        picker.wait_for(ready, Duration::from_secs(10), "Grove remove");
        picker.send(input, "Grove remove");
        let status = picker.wait_for_exit(Duration::from_secs(5), "Grove remove");
        Output {
            status,
            stdout: picker.output(),
            stderr: Vec::new(),
        }
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

    pub fn block_title_generator(&self) -> PathBuf {
        let gate = self._root.path().join("title.block");
        fs::write(&gate, "blocked").expect("create title generator gate");
        gate
    }

    pub fn release_title_generator(&self, gate: &Path) {
        fs::remove_file(gate).expect("release title generator");
    }

    pub fn change_record(&self, capsule: &Path) -> serde_json::Value {
        serde_json::from_slice(&fs::read(capsule.join("change.json")).expect("read change record"))
            .expect("valid change record")
    }

    pub fn wait_for_agent_log(&self, expected: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.agent_log().contains(expected) {
            assert!(
                Instant::now() < deadline,
                "agent log never contained {expected:?}\n{}",
                self.agent_log()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn wait_for_change_title(&self, capsule: &Path, expected: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.change_record(capsule)["title"] != expected {
            assert!(
                Instant::now() < deadline,
                "change title never became {expected:?}: {}",
                self.change_record(capsule)
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn wait_for_session_content(&self, expected: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let found = self.pi_session_files().into_iter().any(|path| {
                fs::read_to_string(path).is_ok_and(|session| session.contains(expected))
            });
            if found {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "native Pi session never contained {expected:?}"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn start_blocking_new(&self) -> (Child, PathBuf) {
        let gate = self._root.path().join("agent.block");
        fs::write(&gate, "blocked").expect("create agent block gate");
        let mut command = self.compiled_grove(&self.repo);
        command.stdout(Stdio::null()).stderr(Stdio::null());
        let child = command
            .arg("new")
            .env("GROVE_TEST_AGENT_BLOCK", &gate)
            .spawn()
            .expect("start managed Grove change");
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.agent_log().contains("mode=interactive") || self.change_capsules().is_empty() {
            assert!(
                Instant::now() < deadline,
                "managed Pi did not start\n{}",
                self.agent_log()
            );
            thread::sleep(Duration::from_millis(20));
        }
        (child, gate)
    }

    pub fn release_blocking_agent(&self, mut child: Child, gate: &Path) {
        fs::remove_file(gate).expect("release agent block gate");
        let status = child.wait().expect("wait for managed Grove change");
        assert!(status.success(), "managed Grove change failed: {status}");
    }

    pub fn navigation_exists(&self) -> bool {
        self.navigation.exists()
    }

    pub fn grove_runtime_exists(&self) -> bool {
        self.home.join(".grove/runtime").exists()
    }

    pub fn pi_session_files(&self) -> Vec<PathBuf> {
        self.change_capsules()
            .into_iter()
            .flat_map(|capsule| {
                fs::read_dir(capsule.join("sessions/pi"))
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .collect::<Vec<_>>()
            })
            .filter(|path| path.extension() == Some(OsStr::new("jsonl")))
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
            .arg(program);
        self.configure_grove(&mut command, directory);
        command
    }

    fn compiled_grove(&self, directory: &Path) -> Command {
        let binary = assert_cmd::Command::cargo_bin("grove")
            .expect("compiled grove binary")
            .get_program()
            .to_owned();
        let mut command = Command::new(binary);
        self.configure_grove(&mut command, directory);
        command
    }

    fn configure_grove(&self, command: &mut Command, directory: &Path) {
        command
            .current_dir(directory)
            .env("HOME", &self.home)
            .env_remove("XDG_CONFIG_HOME")
            .env("GIT_CONFIG_GLOBAL", &self.git_config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GROVE_DIRECTIVE_CD_FILE", &self.navigation)
            .env("GROVE_TEST_AGENT_LOG", &self.agent_log)
            .env("PATH", self.test_path());
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
