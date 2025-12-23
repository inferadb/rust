//! Common types for control plane operations.

use serde::{Deserialize, Serialize};

/// A paginated response containing items of type T.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    /// The items in this page.
    pub items: Vec<T>,
    /// Pagination information.
    pub page_info: PageInfo,
}

impl<T> Page<T> {
    /// Returns `true` if this page is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of items in this page.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if there are more pages available.
    pub fn has_next(&self) -> bool {
        self.page_info.has_next
    }

    /// Returns the cursor for the next page, if available.
    pub fn next_cursor(&self) -> Option<&str> {
        self.page_info.next_cursor.as_deref()
    }
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            page_info: PageInfo::default(),
        }
    }
}

/// Pagination information for a page of results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageInfo {
    /// Whether there are more pages available.
    pub has_next: bool,
    /// Cursor for fetching the next page.
    pub next_cursor: Option<String>,
    /// Total count of items (if available).
    pub total_count: Option<u64>,
}

/// Sort order for list operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    /// Ascending order (oldest first, A-Z).
    #[default]
    Ascending,
    /// Descending order (newest first, Z-A).
    Descending,
}

impl SortOrder {
    /// Returns `true` if this is ascending order.
    pub fn is_ascending(&self) -> bool {
        matches!(self, SortOrder::Ascending)
    }

    /// Returns `true` if this is descending order.
    pub fn is_descending(&self) -> bool {
        matches!(self, SortOrder::Descending)
    }

    /// Returns the string representation for API queries.
    pub fn as_str(&self) -> &'static str {
        match self {
            SortOrder::Ascending => "asc",
            SortOrder::Descending => "desc",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_empty() {
        let page: Page<String> = Page::default();
        assert!(page.is_empty());
        assert_eq!(page.len(), 0);
        assert!(!page.has_next());
        assert!(page.next_cursor().is_none());
    }

    #[test]
    fn test_page_with_items() {
        let page = Page {
            items: vec!["item1".to_string(), "item2".to_string()],
            page_info: PageInfo {
                has_next: true,
                next_cursor: Some("cursor_abc".to_string()),
                total_count: Some(10),
            },
        };
        assert!(!page.is_empty());
        assert_eq!(page.len(), 2);
        assert!(page.has_next());
        assert_eq!(page.next_cursor(), Some("cursor_abc"));
    }

    #[test]
    fn test_sort_order() {
        assert!(SortOrder::Ascending.is_ascending());
        assert!(!SortOrder::Ascending.is_descending());
        assert!(!SortOrder::Descending.is_ascending());
        assert!(SortOrder::Descending.is_descending());
        assert_eq!(SortOrder::default(), SortOrder::Ascending);
    }
}
