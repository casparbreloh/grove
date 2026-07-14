use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tempfile::TempDir;

pub struct TestRepo {
    _root: TempDir,
    repo: PathBuf,
    home: PathBuf,
    git_config: PathBuf,
    navigation: PathBuf,
}

impl TestRepo {
    pub fn new() -> Self {
        let root = tempfile::tempdir().expect("create test directory");
        let repo = root.path().join("repo");
        let home = root.path().join("home");
        let git_config = root.path().join("gitconfig");
        let navigation = root.path().join("navigation");
        fs::create_dir(&home).expect("create test home");

        let fixture = Self {
            _root: root,
            repo,
            home,
            git_config,
            navigation,
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

    pub fn grove_from(&self, directory: &Path) -> assert_cmd::Command {
        let mut command = assert_cmd::Command::cargo_bin("grove").expect("compiled grove binary");
        command
            .current_dir(directory)
            .env("HOME", &self.home)
            .env("GIT_CONFIG_GLOBAL", &self.git_config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GROVE_DIRECTIVE_CD_FILE", &self.navigation);
        command
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
