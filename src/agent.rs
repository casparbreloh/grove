use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, de::DeserializeOwned};

const DEFAULT_AGENT: &str = "pi";

pub struct Agent {
    command: Vec<String>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
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
    pub fn load(project: &Path) -> Result<Self> {
        let global_path = global_config_path();
        let global: Config = global_path
            .as_deref()
            .map(read_config)
            .transpose()?
            .unwrap_or_default();
        let project: ProjectConfig = read_config(&project.join("grove.toml"))?;
        let selected = project
            .agent
            .as_deref()
            .or(global.agent.as_deref())
            .unwrap_or(DEFAULT_AGENT);
        let config = global
            .agents
            .get(selected)
            .cloned()
            .or_else(|| preset(selected))
            .with_context(|| format!("agent '{selected}' is not configured"))?;
        if config.command.is_empty() {
            bail!("agent '{selected}' requires command arguments");
        }
        Ok(Self {
            command: config.command,
        })
    }

    pub fn launch(&self, worktree: &Path, task: Option<&str>) -> Result<()> {
        let status = command(&self.command, task)?
            .current_dir(worktree)
            .env_remove("GROVE_DIRECTIVE_CD_FILE")
            .status()
            .context("failed to launch the configured agent")?;
        if !status.success() {
            bail!("agent exited with {status}");
        }
        Ok(())
    }
}

fn read_config<T: Default + DeserializeOwned>(path: &Path) -> Result<T> {
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
    let command: &[&str] = match name {
        "pi" => &["pi", "{prompt}"],
        "claude" | "claude-code" => &["claude", "{prompt}"],
        "codex" => &["codex", "{prompt}"],
        _ => return None,
    };
    Some(AgentConfig {
        command: command.iter().map(|value| (*value).to_owned()).collect(),
    })
}

fn command(template: &[String], prompt: Option<&str>) -> Result<Command> {
    let mut arguments = Vec::new();
    let mut replaced = false;
    for argument in template {
        if argument == "{prompt}" {
            replaced = true;
            if let Some(prompt) = prompt {
                arguments.push(prompt.to_owned());
            }
        } else {
            arguments.push(argument.clone());
        }
    }
    if prompt.is_some() && !replaced {
        arguments.push(prompt.unwrap_or_default().to_owned());
    }
    if arguments.is_empty() {
        bail!("agent command has no executable");
    }
    let mut command = Command::new(&arguments[0]);
    command.args(&arguments[1..]);
    Ok(command)
}
