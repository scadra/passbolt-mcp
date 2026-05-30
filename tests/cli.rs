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

use std::process::Command;

fn passbolt_mcp(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_passbolt-mcp"))
        .args(args)
        .output()
        .unwrap();
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn version() {
    let (ok, stdout) = passbolt_mcp(&["--version"]);
    assert!(ok);
    assert_eq!(
        stdout,
        format!("passbolt-mcp {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help() {
    let (ok, stdout) = passbolt_mcp(&["--help"]);
    assert!(ok);
    assert!(stdout.contains("Usage: passbolt-mcp"));
}
