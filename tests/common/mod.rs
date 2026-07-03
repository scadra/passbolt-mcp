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

//! The fake Passbolt of the SDK tests (`passbolt-sdk`, `tests/it/common`, at the version this
//! crate uses), behind a local HTTP server: `passbolt-mcp` runs against it as against Passbolt.
#![allow(dead_code, unused_imports)] // Each test crate uses a part.

pub mod data;
pub mod fake;
pub mod keys;

pub use fake::FakePassbolt;
pub use keys::{ADA, BETTY, CAROL, ERIN, METADATA, SERVER};

pub const BASE_URL: &str = "https://passbolt.test";
pub const CSRF_TOKEN: &str = "csrf-test-token";

// Must match tests/fixtures/data.
pub const ADA_ID: &str = "8b6c3a10-0000-4000-8000-00000000a0da";
pub const BETTY_ID: &str = "8b6c3a10-0000-4000-8000-000000000be7";
pub const CAROL_ID: &str = "8b6c3a10-0000-4000-8000-0000000ca201";
pub const ERIN_ID: &str = "8b6c3a10-0000-4000-8000-00000000e121";
pub const GROUP_ID: &str = "9c7d4b20-0000-4000-8000-000000000960";
pub const RESOURCE_ID: &str = "7a5b2c30-0000-4000-8000-0000000000e1";
pub const FOLDER_ID: &str = "5f6e7d8c-0000-4000-8000-0000000000f1";
