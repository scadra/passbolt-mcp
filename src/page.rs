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

//! Pages of the list tools: an agent asks for a few items at a time, not thousands.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ToolError;

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;

/// The page arguments of a list tool.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct PageArgs {
    /// At most this many items: 50 by default, 200 at most.
    pub limit: Option<usize>,
    /// The `next_cursor` of the previous page, for the next one.
    pub cursor: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Page<T> {
    pub items: Vec<T>,
    /// How many there are in all.
    pub total: usize,
    /// To get the next page; absent on the last one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl PageArgs {
    /// The page of `items` these arguments ask for.
    pub fn page<T>(&self, items: Vec<T>) -> Result<Page<T>, ToolError> {
        let limit = self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let start = match &self.cursor {
            None => 0,
            Some(cursor) => cursor
                .parse::<usize>()
                .map_err(|_| ToolError::invalid_argument(format!("unknown cursor {cursor}")))?,
        };
        let total = items.len();
        let end = start.saturating_add(limit).min(total);
        Ok(Page {
            items: items.into_iter().skip(start).take(limit).collect(),
            total,
            next_cursor: (end < total).then(|| end.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(limit: Option<usize>, cursor: Option<&str>) -> PageArgs {
        PageArgs {
            limit,
            cursor: cursor.map(Into::into),
        }
    }

    #[test]
    fn pages() {
        let items: Vec<u32> = (0..5).collect();
        let first = args(Some(2), None).page(items.clone()).unwrap();
        assert_eq!(first.items, [0, 1]);
        assert_eq!(first.total, 5);
        let cursor = first.next_cursor.unwrap();
        let second = args(Some(2), Some(&cursor)).page(items.clone()).unwrap();
        assert_eq!(second.items, [2, 3]);
        let last = args(Some(2), second.next_cursor.as_deref())
            .page(items)
            .unwrap();
        assert_eq!(last.items, [4]);
        assert_eq!(last.next_cursor, None);
    }

    #[test]
    fn limits() {
        let items: Vec<u32> = (0..300).collect();
        assert_eq!(
            PageArgs::default().page(items.clone()).unwrap().items.len(),
            50
        );
        assert_eq!(
            args(Some(1000), None)
                .page(items.clone())
                .unwrap()
                .items
                .len(),
            200
        );
        assert_eq!(args(Some(0), None).page(items).unwrap().items.len(), 1);
    }

    #[test]
    fn past_the_end() {
        let page = args(None, Some("10")).page(vec![1, 2]).unwrap();
        assert_eq!(page.items.len(), 0);
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn bad_cursor() {
        assert_eq!(
            args(None, Some("abc")).page(vec![1]).unwrap_err().code,
            "invalid_argument"
        );
    }
}
