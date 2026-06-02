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

//! The passphrase of the MCP user: from a file or a command, never from the command line nor
//! the config file, which end up in shell histories, process lists and backups.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use secrecy::SecretString;
use zeroize::Zeroizing;

use crate::config::PassboltConfig;

/// `passphrase_file` or `passphrase_command`, exactly one.
pub fn read(config: &PassboltConfig) -> Result<SecretString> {
    match (&config.passphrase_file, &config.passphrase_command) {
        (Some(path), None) => from_file(path),
        (None, Some(command)) => from_command(command),
        (Some(_), Some(_)) => bail!("set passphrase_file or passphrase_command, not both"),
        (None, None) => bail!("no passphrase: set passphrase_file or passphrase_command"),
    }
}

/// The first line of the file.
pub fn from_file(path: &Path) -> Result<SecretString> {
    let text = Zeroizing::new(
        std::fs::read_to_string(path)
            .with_context(|| format!("cannot read the passphrase from {}", path.display()))?,
    );
    first_line(&text)
}

/// The first line the command prints, as `pass show` prints the password.
///
/// stdin is closed: in stdio, it carries the MCP messages. stderr is kept, for a prompt.
pub fn from_command(command: &[String]) -> Result<SecretString> {
    let (program, args) = command
        .split_first()
        .context("passphrase_command is empty")?;
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("cannot run {program}"))?;
    let stdout = Zeroizing::new(output.stdout);
    if !output.status.success() {
        bail!("{program} failed: {}", output.status);
    }
    let text =
        Zeroizing::new(String::from_utf8(stdout.to_vec()).context("the passphrase is not UTF-8")?);
    first_line(&text)
}

fn first_line(text: &str) -> Result<SecretString> {
    let line = text.lines().next().unwrap_or_default();
    if line.is_empty() {
        bail!("the passphrase is empty");
    }
    Ok(SecretString::from(line))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use secrecy::ExposeSecret;

    use super::*;

    fn file(text: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(text.as_bytes()).unwrap();
        file
    }

    #[test]
    fn first_line_of_a_file() {
        let file = file("correct horse battery staple\r\nsecond line\n");
        assert_eq!(
            from_file(file.path()).unwrap().expose_secret(),
            "correct horse battery staple"
        );
    }

    #[test]
    fn empty_file() {
        assert!(from_file(file("\n").path()).is_err());
    }

    fn command(args: &[&str]) -> Result<SecretString> {
        from_command(&args.iter().map(ToString::to_string).collect::<Vec<_>>())
    }

    #[test]
    fn first_line_of_a_command() {
        assert_eq!(
            command(&["printf", "s3cret\\nurl: x\\n"])
                .unwrap()
                .expose_secret(),
            "s3cret"
        );
    }

    #[test]
    fn failing_command() {
        assert!(command(&["false"]).is_err());
        assert!(command(&[]).is_err());
    }

    #[test]
    fn both() {
        let config = PassboltConfig {
            passphrase_file: Some("p".into()),
            passphrase_command: Some(vec!["p".into()]),
            ..PassboltConfig::default()
        };
        assert!(read(&config).is_err());
    }

    #[test]
    fn none() {
        assert!(read(&PassboltConfig::default()).is_err());
    }
}
