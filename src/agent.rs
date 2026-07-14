use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, de::DeserializeOwned};

use crate::{git::Git, runtime};

const DEFAULT_AGENT: &str = "pi";

pub struct Agent {
    name: String,
    command: Vec<String>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalConfig {
    agent: Option<String>,
    #[serde(default)]
    agents: HashMap<String, AgentConfig>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectConfig {
    agent: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentConfig {
    command: Vec<String>,
}

impl Agent {
    pub fn load(git: &Git, requested: Option<&str>) -> Result<Self> {
        let global_path = global_config_path();
        let global: GlobalConfig = global_path
            .as_deref()
            .map(read_config)
            .transpose()?
            .unwrap_or_default();
        let project: ProjectConfig = read_config(&git.project_root()?.join("grove.toml"))?;
        let name = requested
            .or(project.agent.as_deref())
            .or(global.agent.as_deref())
            .unwrap_or(DEFAULT_AGENT);
        let config = global
            .agents
            .get(name)
            .cloned()
            .or_else(|| preset(name))
            .with_context(|| format!("agent '{name}' is not configured"))?;
        if config.command.is_empty() {
            bail!("agent '{name}' requires command arguments");
        }
        if config
            .command
            .iter()
            .any(|argument| argument.contains("{prompt}"))
        {
            bail!("{{prompt}} is no longer supported; remove it from the agent command");
        }
        Ok(Self {
            name: name.to_owned(),
            command: config.command,
        })
    }

    pub fn attach(self, git: &Git) -> Result<()> {
        runtime::attach(&git.worktree_identity()?, &self.name, self.command)
    }
}

fn read_config<T: Default + DeserializeOwned>(path: &std::path::Path) -> Result<T> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))
}

fn global_config_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|root| root.join("grove/grove.toml"))
}

fn preset(name: &str) -> Option<AgentConfig> {
    let executable = match name {
        "pi" => "pi",
        "claude" | "claude-code" => "claude",
        "codex" => "codex",
        _ => return None,
    };
    Some(AgentConfig {
        command: vec![executable.to_owned()],
    })
}
