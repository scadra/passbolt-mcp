// Passbolt ~ Open source password manager for teams
// Copyright (c) Passbolt SA (https://www.passbolt.com)
//
// Licensed under GNU Affero General Public License version 3 of the or any later version.
// For full copyright and license information, please see the LICENSE
// Redistributions of files must retain the above copyright notice.
//
// @copyright     Copyright (c) Passbolt SA (https://www.passbolt.com)
// @license       https://opensource.org/licenses/AGPL-3.0 AGPL License
// @link          https://www.passbolt.com Passbolt(tm)
// @since         0.1.0

use std::collections::{BTreeMap, BTreeSet};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, ensure, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use zeroize::Zeroizing;

use crate::error::ToolError;
use crate::vault::scrub;

pub const SECRET: &str = "{secret}";
pub const RESOURCE: &str = "passbolt:";
const OUTPUT_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandConfig {
    pub run: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub description: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

const fn default_timeout() -> u64 {
    60
}

#[derive(Debug, PartialEq, Eq)]
pub enum EnvValue<'a> {
    Secret,
    Resource(&'a str),
    Plain(&'a str),
}

impl CommandConfig {
    pub fn check(&self, name: &str) -> Result<()> {
        ensure!(!self.run.is_empty(), "commands.{name}.run is empty");
        for arg in &self.run {
            ensure!(
                !arg.contains(SECRET) && !arg.contains(RESOURCE),
                "commands.{name}.run: a secret goes in env, never in the arguments, which other \
                 processes can read"
            );
        }
        for (variable, value) in &self.env {
            if let EnvValue::Resource(id) = env_value(value) {
                ensure!(
                    !id.is_empty(),
                    "commands.{name}.env.{variable}: passbolt: needs a resource id"
                );
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn takes_secret(&self) -> bool {
        self.env
            .values()
            .any(|value| env_value(value) == EnvValue::Secret)
    }

    #[must_use]
    pub fn placeholders(&self) -> BTreeSet<&str> {
        self.run.iter().flat_map(|arg| placeholders(arg)).collect()
    }

    pub fn args(&self, given: &BTreeMap<String, String>) -> Result<Vec<String>, ToolError> {
        let wanted = self.placeholders();
        if let Some(unknown) = given.keys().find(|name| !wanted.contains(name.as_str())) {
            return Err(ToolError::invalid_argument(format!(
                "the command takes no argument {unknown}; it takes: {}",
                wanted.into_iter().collect::<Vec<_>>().join(", ")
            )));
        }
        if let Some(missing) = wanted.iter().find(|name| !given.contains_key(**name)) {
            return Err(ToolError::invalid_argument(format!(
                "the command needs the argument {missing}"
            )));
        }
        Ok(self.run.iter().map(|arg| fill(arg, given)).collect())
    }
}

#[must_use]
pub fn env_value(value: &str) -> EnvValue<'_> {
    if value == SECRET {
        EnvValue::Secret
    } else if let Some(id) = value.strip_prefix(RESOURCE) {
        EnvValue::Resource(id)
    } else {
        EnvValue::Plain(value)
    }
}

pub fn check_all(commands: &BTreeMap<String, CommandConfig>) -> Result<()> {
    for (name, command) in commands {
        if name.is_empty() {
            bail!("a command needs a name");
        }
        command.check(name)?;
    }
    Ok(())
}

fn placeholders(arg: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = arg;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else { break };
        let name = &after[..end];
        if !name.is_empty()
            && name
                .chars()
                .all(|char| char.is_ascii_alphanumeric() || char == '_')
        {
            found.push(name);
        }
        rest = &after[end + 1..];
    }
    found
}

fn fill(arg: &str, given: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = arg;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        if let Some((value, end)) = after
            .find('}')
            .and_then(|end| Some((given.get(&after[..end])?, end)))
        {
            out.push_str(value);
            rest = &after[end + 1..];
        } else {
            out.push('{');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "`run_with_secret`.")]
pub struct RunWithSecret {
    #[schemars(description = "The name of a command of the configuration, as `whoami` lists it.")]
    pub command: String,
    #[schemars(
        description = "A handle from `get_secret` or `generate_password`, or the id of a resource, whose password \
        the command gets; for the commands that take one."
    )]
    pub secret: Option<String>,
    #[schemars(description = "The value of each placeholder of the command, by name.")]
    #[serde(default)]
    pub args: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "What the command did; every secret it got is replaced with `[secret]`.")]
pub struct CommandOutput {
    pub command: String,
    #[schemars(description = "Absent when the command was stopped, after its timeout.")]
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    #[schemars(description = "The output was longer, and was cut.")]
    pub truncated: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CommandView {
    pub name: String,
    pub description: Option<String>,
    #[schemars(description = "The arguments the agent gives, by name.")]
    pub args: Vec<String>,
    #[schemars(description = "It needs `secret`: a handle or a resource id.")]
    pub takes_secret: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "`list_commands`.")]
pub struct Commands {
    pub commands: Vec<CommandView>,
}

impl CommandView {
    #[must_use]
    pub fn new(name: &str, command: &CommandConfig) -> Self {
        Self {
            name: name.into(),
            description: command.description.clone(),
            args: command
                .placeholders()
                .into_iter()
                .map(String::from)
                .collect(),
            takes_secret: command.takes_secret(),
        }
    }
}

pub async fn run(
    name: &str,
    argv: &[String],
    env: &[(String, Zeroizing<String>)],
    plain: &[(&String, &str)],
    timeout: Duration,
) -> Result<CommandOutput, ToolError> {
    let mut command = tokio::process::Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for (variable, value) in plain {
        command.env(variable, value);
    }
    for (variable, value) in env {
        command.env(variable, value.as_str());
    }
    let mut child = command.spawn().map_err(|error| {
        ToolError::invalid_argument(format!("the command {name} could not start: {error}"))
    })?;
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let read = async {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let (read_out, read_err) =
            tokio::join!(stdout.read_to_end(&mut out), stderr.read_to_end(&mut err));
        read_out.and(read_err)?;
        let status = child.wait().await?;
        std::io::Result::Ok((status, out, err))
    };
    let (exit_code, out, err) = match tokio::time::timeout(timeout, read).await {
        Ok(Ok((status, out, err))) => (status.code(), out, err),
        Ok(Err(error)) => {
            return Err(ToolError::invalid_argument(format!(
                "the command {name} failed: {error}"
            )))
        }
        Err(_) => (None, Vec::new(), b"stopped: over its timeout".to_vec()),
    };
    let secrets: Vec<Zeroizing<String>> = env.iter().map(|(_, value)| value.clone()).collect();
    let (stdout, cut_out) = cut(&out);
    let (stderr, cut_err) = cut(&err);
    Ok(CommandOutput {
        command: name.into(),
        exit_code,
        stdout: scrub(&stdout, &secrets),
        stderr: scrub(&stderr, &secrets),
        truncated: cut_out || cut_err,
    })
}

fn cut(bytes: &[u8]) -> (Zeroizing<String>, bool) {
    let text = Zeroizing::new(String::from_utf8_lossy(bytes).into_owned());
    if text.len() <= OUTPUT_LIMIT {
        return (text, false);
    }
    let mut end = OUTPUT_LIMIT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (Zeroizing::new(text[..end].to_owned()), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(run: &[&str], env: &[(&str, &str)]) -> CommandConfig {
        CommandConfig {
            run: run.iter().map(|arg| (*arg).into()).collect(),
            env: env
                .iter()
                .map(|(name, value)| ((*name).into(), (*value).into()))
                .collect(),
            description: None,
            timeout_seconds: 5,
        }
    }

    #[test]
    fn checked() {
        assert!(
            command(&["psql", "-c", "{query}"], &[("PGPASSWORD", SECRET)])
                .check("psql")
                .is_ok()
        );
        assert!(command(&[], &[]).check("empty").is_err());
        assert!(command(&["echo", "{secret}"], &[]).check("echo").is_err());
        assert!(command(&["echo", "passbolt:r1"], &[])
            .check("echo")
            .is_err());
        assert!(command(&["env"], &[("X", "passbolt:")])
            .check("env")
            .is_err());
    }

    #[test]
    fn env_values() {
        assert_eq!(env_value("{secret}"), EnvValue::Secret);
        assert_eq!(env_value("passbolt:r1"), EnvValue::Resource("r1"));
        assert_eq!(env_value("require"), EnvValue::Plain("require"));
    }

    #[test]
    fn arguments() {
        let psql = command(&["psql", "-c", "{query}", "--db={db}", "{x"], &[]);
        assert_eq!(
            psql.placeholders().into_iter().collect::<Vec<_>>(),
            ["db", "query"]
        );
        let given: BTreeMap<String, String> = [
            ("query".to_string(), "select '{db}'".to_string()),
            ("db".to_string(), "app".to_string()),
        ]
        .into();
        assert_eq!(
            psql.args(&given).unwrap(),
            ["psql", "-c", "select '{db}'", "--db=app", "{x"]
        );
        let missing = BTreeMap::from([("query".to_string(), "x".to_string())]);
        assert!(psql.args(&missing).unwrap_err().message.contains("db"));
        let mut extra = given;
        extra.insert("shell".into(), "x".into());
        assert!(psql.args(&extra).unwrap_err().message.contains("shell"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn runs_and_scrubs() {
        let argv = [
            "sh",
            "-c",
            "echo \"$TOKEN\"; printf %s \"$TOKEN\" | base64; echo $MODE >&2; exit 3",
        ]
        .map(String::from);
        let output = run(
            "echo",
            &argv,
            &[("TOKEN".into(), Zeroizing::new("hunter2".into()))],
            &[(&"MODE".to_string(), "plain")],
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert_eq!(output.stdout, "[secret]\n[secret]\n");
        assert_eq!(output.stderr, "plain\n");
        assert_eq!(output.exit_code, Some(3));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn timeout() {
        let argv = ["sleep", "5"].map(String::from);
        let output = run("sleep", &argv, &[], &[], Duration::from_millis(100))
            .await
            .unwrap();
        assert_eq!(output.exit_code, None);
        assert!(output.stderr.contains("timeout"));
    }
}
