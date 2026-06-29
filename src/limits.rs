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

//! `[limits]`: calls a minute, counted over the last sixty seconds.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::LimitsConfig;
use crate::rules::{Access, Refusal};

const WINDOW: Duration = Duration::from_secs(60);

struct Window {
    max: usize,
    calls: Mutex<VecDeque<Instant>>,
}

impl Window {
    fn new(max: u32) -> Self {
        Self {
            max: max as usize,
            calls: Mutex::default(),
        }
    }

    /// Room for one more call at `now`.
    fn has_room(&self, now: Instant) -> bool {
        let mut calls = self
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while calls
            .front()
            .is_some_and(|call| now.duration_since(*call) >= WINDOW)
        {
            calls.pop_front();
        }
        calls.len() < self.max
    }

    fn count(&self, now: Instant) {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(now);
    }
}

pub struct Limits {
    calls: Window,
    secrets: Window,
}

impl Limits {
    #[must_use]
    pub fn new(config: &LimitsConfig) -> Self {
        Self {
            calls: Window::new(config.calls_per_minute),
            secrets: Window::new(config.secrets_per_minute),
        }
    }

    /// Count a call needing `needs`, unless it is over a limit.
    pub fn take(&self, needs: Option<Access>) -> Result<(), Refusal> {
        self.take_at(needs, Instant::now())
    }

    fn take_at(&self, needs: Option<Access>, now: Instant) -> Result<(), Refusal> {
        if !self.calls.has_room(now) {
            return Err(Refusal::RateLimited("calls_per_minute"));
        }
        let secret = needs == Some(Access::Secrets);
        if secret && !self.secrets.has_room(now) {
            return Err(Refusal::RateLimited("secrets_per_minute"));
        }
        self.calls.count(now);
        if secret {
            self.secrets.count(now);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(calls: u32, secrets: u32) -> Limits {
        Limits::new(&LimitsConfig {
            calls_per_minute: calls,
            secrets_per_minute: secrets,
        })
    }

    #[test]
    fn secrets() {
        let limits = limits(10, 2);
        let now = Instant::now();
        assert!(limits.take_at(Some(Access::Secrets), now).is_ok());
        assert!(limits.take_at(Some(Access::Secrets), now).is_ok());
        assert_eq!(
            limits.take_at(Some(Access::Secrets), now),
            Err(Refusal::RateLimited("secrets_per_minute"))
        );
        // Reading metadata still works.
        assert!(limits.take_at(Some(Access::Read), now).is_ok());
        // A minute later, room again.
        assert!(limits
            .take_at(Some(Access::Secrets), now + Duration::from_secs(61))
            .is_ok());
    }

    #[test]
    fn every_call() {
        let limits = limits(2, 10);
        let now = Instant::now();
        assert!(limits.take_at(Some(Access::Read), now).is_ok());
        assert!(limits.take_at(None, now).is_ok());
        assert_eq!(
            limits.take_at(Some(Access::Secrets), now),
            Err(Refusal::RateLimited("calls_per_minute"))
        );
    }
}
