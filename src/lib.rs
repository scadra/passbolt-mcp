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

//! An MCP server for Passbolt: an AI agent works with Passbolt through a user of its own.

pub mod config;
pub mod error;
pub mod page;
pub mod passphrase;
pub mod read;
pub mod rules;
pub mod scope;
pub mod server;
pub mod server_key;
pub mod session;
