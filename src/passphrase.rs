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

use anyhow::{bail, Context, Result};
use secrecy::SecretString;
use zeroize::Zeroizing;

use crate::config::PassboltConfig;

/// `passphrase_file`.
pub fn read(config: &PassboltConfig) -> Result<SecretString> {
    match &config.passphrase_file {
        Some(path) => from_file(path),
        None => bail!("no passphrase: set passphrase_file in [passbolt]"),
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

    #[test]
    fn none() {
        assert!(read(&PassboltConfig::default()).is_err());
    }
}
