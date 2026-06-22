//! Pagination types for repository listings. Supports unbounded (All) and
//! bounded (Offset, Cursor) pages. `Listing<T>` carries items and an optional
//! continuation cursor; the cursor is stable across multiple requests and
//! orders results consistently.

use serde::{Deserialize, Serialize};

/// A page specification for repository `list` operations.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Page {
    /// Fetch all items without pagination.
    All,
    /// Fetch a bounded offset-based page: `limit` items starting at `offset`.
    Offset { limit: u32, offset: u32 },
    /// Fetch a bounded cursor-based page: `limit` items after a cursor (or from the start).
    Cursor { after: Option<String>, limit: u32 },
}

impl Page {
    /// Create an offset-based page.
    pub fn offset(limit: u32, offset: u32) -> Self {
        Page::Offset { limit, offset }
    }

    /// Create a cursor-based page starting from the beginning.
    pub fn cursor_from_start(limit: u32) -> Self {
        Page::Cursor { after: None, limit }
    }

    /// Create a cursor-based page starting after a cursor.
    pub fn cursor_after(cursor: impl Into<String>, limit: u32) -> Self {
        Page::Cursor {
            after: Some(cursor.into()),
            limit,
        }
    }
}

/// A page of results with an optional continuation cursor for the next page.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Listing<T> {
    /// The items in this page.
    pub items: Vec<T>,
    /// The cursor for the next page, if more items exist after this page.
    pub next: Option<String>,
}

impl<T> Listing<T> {
    /// Create a listing with no items and no continuation.
    pub fn empty() -> Self {
        Listing {
            items: Vec::new(),
            next: None,
        }
    }

    /// Create a listing with items and an optional continuation cursor.
    pub fn new(items: Vec<T>, next: Option<String>) -> Self {
        Listing { items, next }
    }

    /// Return true if there are more items after this page.
    pub fn has_next(&self) -> bool {
        self.next.is_some()
    }

    /// Map a listing's items to a different type, preserving the continuation cursor.
    pub fn map<U, F: FnOnce(Vec<T>) -> Vec<U>>(self, f: F) -> Listing<U> {
        Listing {
            items: f(self.items),
            next: self.next,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Scenario: A bounded page returns a continuation cursor
    #[test]
    fn cursor_page_carries_after_and_limit() {
        let page = Page::cursor_after("cursor_123", 10);
        match page {
            Page::Cursor { after, limit } => {
                assert_eq!(after, Some("cursor_123".to_string()));
                assert_eq!(limit, 10);
            }
            _ => panic!("Expected Cursor variant"),
        }
    }

    #[test]
    fn cursor_page_from_start_has_none_cursor() {
        let page = Page::cursor_from_start(10);
        match page {
            Page::Cursor { after, limit } => {
                assert_eq!(after, None);
                assert_eq!(limit, 10);
            }
            _ => panic!("Expected Cursor variant"),
        }
    }

    #[test]
    fn listing_with_next_cursor_reports_has_next() {
        let listing = Listing::new(vec![1, 2, 3], Some("next_cursor".to_string()));
        assert!(listing.has_next());
        assert_eq!(listing.next, Some("next_cursor".to_string()));
    }

    #[test]
    fn listing_without_next_cursor_reports_no_next() {
        let listing = Listing::new(vec![1, 2, 3], None);
        assert!(!listing.has_next());
        assert_eq!(listing.next, None);
    }

    // Scenario: Unbounded listing is explicit
    #[test]
    fn all_page_variant_exists() {
        let page = Page::All;
        assert_eq!(page, Page::All);
    }

    #[test]
    fn offset_page_carries_limit_and_offset() {
        let page = Page::offset(20, 40);
        match page {
            Page::Offset { limit, offset } => {
                assert_eq!(limit, 20);
                assert_eq!(offset, 40);
            }
            _ => panic!("Expected Offset variant"),
        }
    }

    #[test]
    fn listing_empty_has_no_items_and_no_next() {
        let listing: Listing<i32> = Listing::empty();
        assert!(listing.items.is_empty());
        assert!(!listing.has_next());
    }

    #[test]
    fn listing_map_preserves_continuation_cursor() {
        let listing = Listing::new(vec![1, 2, 3], Some("next".to_string()));
        let mapped = listing.map(|nums| nums.iter().map(|n| n * 2).collect());
        assert_eq!(mapped.items, vec![2, 4, 6]);
        assert_eq!(mapped.next, Some("next".to_string()));
    }

    #[test]
    fn page_variants_are_distinct() {
        let all = Page::All;
        let offset = Page::offset(10, 0);
        let cursor = Page::cursor_from_start(10);
        assert_ne!(all, offset);
        assert_ne!(all, cursor);
        assert_ne!(offset, cursor);
    }

    #[test]
    fn listing_serializes_and_deserializes() {
        let listing = Listing::new(vec![1, 2, 3], Some("cursor".to_string()));
        let json = serde_json::to_string(&listing).expect("serialize");
        let deserialized: Listing<i32> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, listing);
    }

    #[test]
    fn page_serializes_and_deserializes() {
        let page = Page::cursor_after("abc", 5);
        let json = serde_json::to_string(&page).expect("serialize");
        let deserialized: Page = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, page);
    }
}
