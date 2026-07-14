use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result, bail};
use rmux_client::{
    AttachTransition, AutoStartConfig, ConnectResult, attach_terminal_with_initial_bytes,
    ensure_server_running_with_config,
};
use rmux_proto::{
    OptionName, ProcessCommand, Response, ScopeSelector, SessionName, SetOptionMode,
    request::{KillSessionRequest, ListSessionsRequest, NewSessionExtRequest},
};
use rmux_server::{DaemonConfig, ServerDaemon};

use crate::git::WorktreeIdentity;

pub(crate) fn attach(
    identity: &WorktreeIdentity,
    agent_name: &str,
    command: Vec<String>,
) -> Result<()> {
    let endpoint = endpoint()?;
    if let Some(parent) = endpoint.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let config = AutoStartConfig::disabled().with_binary_override(std::env::current_exe()?);
    let mut connection = ensure_server_running_with_config(&endpoint, config)
        .context("failed to start the embedded agent runtime")?;
    let session = session_name(identity, agent_name)?;
    let executable = command.first().cloned().context("agent command is empty")?;
    let request = |attach_if_exists| NewSessionExtRequest {
        session_name: Some(session.clone()),
        working_directory: Some(identity.root.display().to_string()),
        detached: true,
        size: None,
        environment: None,
        group_target: None,
        attach_if_exists,
        detach_other_clients: false,
        kill_other_clients: false,
        flags: None,
        window_name: None,
        print_session_info: false,
        print_format: None,
        command: None,
        process_command: Some(ProcessCommand::Argv(command.clone())),
        client_environment: Some(environment()),
        skip_environment_update: false,
    };
    match connection.new_session_extended(request(false))? {
        Response::NewSession(_) => {}
        Response::Error(response)
            if matches!(response.error, rmux_proto::RmuxError::DuplicateSession(_)) =>
        {
            match connection.new_session_extended(request(true))? {
                Response::NewSession(_) => {}
                Response::Error(response) => {
                    return Err(response.error)
                        .with_context(|| format!("failed to launch agent '{executable}'"));
                }
                response => {
                    bail!("agent runtime returned an unexpected response: {response:?}")
                }
            }
        }
        Response::Error(response) => {
            return Err(response.error)
                .with_context(|| format!("failed to launch agent '{executable}'"));
        }
        response => bail!("agent runtime returned an unexpected response: {response:?}"),
    }
    match connection
        .set_option(
            ScopeSelector::Session(session.clone()),
            OptionName::Status,
            "off".to_owned(),
            SetOptionMode::Replace,
        )
        .context("failed to hide embedded agent runtime chrome")?
    {
        Response::SetOption(_) => {}
        Response::Error(response) => {
            return Err(response.error).context("failed to hide embedded agent runtime chrome");
        }
        response => bail!("agent runtime returned an unexpected response: {response:?}"),
    }
    match connection.begin_attach(session)? {
        AttachTransition::Upgraded(upgrade) => {
            let (stream, initial_bytes) = upgrade.into_parts();
            attach_terminal_with_initial_bytes(stream, initial_bytes)
                .context("failed to attach the agent terminal")
        }
        AttachTransition::Rejected(Response::Error(response)) => Err(response.error.into()),
        AttachTransition::Rejected(response) => {
            bail!("agent runtime rejected terminal attachment: {response:?}")
        }
    }
}

pub(crate) fn run_daemon(args: impl Iterator<Item = OsString>) -> Result<()> {
    let mut args = args;
    let socket = args.next().context("missing embedded runtime endpoint")?;
    let mut ready_fd = None;
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--config-quiet") => {}
            Some("--startup-ready-fd") => {
                ready_fd = Some(
                    args.next()
                        .context("--startup-ready-fd requires a value")?
                        .to_string_lossy()
                        .parse::<i32>()
                        .context("--startup-ready-fd requires an integer")?,
                );
            }
            Some(argument) => bail!("unexpected embedded runtime argument '{argument}'"),
            None => bail!("embedded runtime arguments must be UTF-8"),
        }
    }
    let config = daemon_config(PathBuf::from(socket), ready_fd);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let server = ServerDaemon::new(config).bind().await?;
            server.wait().await
        })?;
    Ok(())
}

pub(crate) fn sessions(identity: &WorktreeIdentity) -> Result<Vec<SessionName>> {
    let endpoint = endpoint()?;
    let ConnectResult::Connected(mut connection) = rmux_client::connect_or_absent(&endpoint)
        .context("failed to inspect embedded agent runtime")?
    else {
        return Ok(Vec::new());
    };
    let response = connection.list_sessions(ListSessionsRequest {
        format: Some("#{session_name}".to_owned()),
        filter: None,
        sort_order: None,
        reversed: false,
    })?;
    let output = match response {
        Response::ListSessions(response) => response.output.stdout,
        Response::Error(response) => return Err(response.error.into()),
        response => bail!("agent runtime returned an unexpected response: {response:?}"),
    };
    let prefix = session_prefix(identity);
    String::from_utf8(output)
        .context("agent runtime returned invalid session names")?
        .lines()
        .filter(|name| name.starts_with(&prefix))
        .map(|name| SessionName::new(name.to_owned()).map_err(Into::into))
        .collect()
}

pub(crate) fn terminate(sessions: Vec<SessionName>) -> Result<()> {
    if sessions.is_empty() {
        return Ok(());
    }
    let endpoint = endpoint()?;
    let ConnectResult::Connected(mut connection) = rmux_client::connect_or_absent(&endpoint)
        .context("failed to connect to embedded agent runtime")?
    else {
        bail!("embedded agent runtime stopped before agent cleanup");
    };
    for session in sessions {
        match connection.kill_session(KillSessionRequest {
            target: session,
            kill_all_except_target: false,
            clear_alerts: false,
        })? {
            Response::KillSession(response) if response.existed => {}
            Response::KillSession(_) => bail!("agent session ended before cleanup completed"),
            Response::Error(response) => return Err(response.error.into()),
            response => bail!("agent runtime returned an unexpected response: {response:?}"),
        }
    }
    Ok(())
}

fn endpoint() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("GROVE_RUNTIME_SOCKET").filter(|path| !path.is_empty()) {
        return Ok(path.into());
    }
    rmux_client::socket_path_for_label("grove").map_err(Into::into)
}

fn session_name(identity: &WorktreeIdentity, agent_name: &str) -> Result<SessionName> {
    let prefix = session_prefix(identity);
    let mut hasher = blake3::Hasher::new();
    hasher.update(agent_name.as_bytes());
    let digest = hasher.finalize().to_hex();
    SessionName::new(format!("{prefix}{}", &digest[..20])).map_err(Into::into)
}

fn session_prefix(identity: &WorktreeIdentity) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(identity.common_dir.as_os_str().as_encoded_bytes());
    hasher.update(&[0]);
    hasher.update(identity.root.as_os_str().as_encoded_bytes());
    let digest = hasher.finalize().to_hex();
    format!("grove-{}-", &digest[..20])
}

fn environment() -> Vec<String> {
    std::env::vars()
        .filter(|(name, _)| name != "GROVE_DIRECTIVE_CD_FILE")
        .map(|(name, value)| format!("{name}={value}"))
        .collect()
}

fn daemon_config(path: PathBuf, ready_fd: Option<i32>) -> DaemonConfig {
    let config = DaemonConfig::new(path);
    #[cfg(target_os = "linux")]
    let config = if let Some(ready_fd) = ready_fd {
        config.with_startup_ready_fd(ready_fd)
    } else {
        config
    };
    #[cfg(not(target_os = "linux"))]
    let _ = ready_fd;
    config
}
