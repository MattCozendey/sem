pub mod blame;
pub mod context;
pub mod diff;
pub mod entities;
pub mod graph;
pub mod impact;
pub mod log;
pub mod setup;

/// Truncate a string to `max_chars` characters (not bytes), appending "..." if truncated.
/// This is safe for multibyte characters (e.g. CJK, emoji).
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{truncated}...")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_str;

    #[test]
    fn ascii_short_string_unchanged() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn ascii_exact_length_unchanged() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn ascii_truncated_with_ellipsis() {
        // 6 chars > max 5, so take 2 chars + "..."
        assert_eq!(truncate_str("abcdef", 5), "ab...");
    }

    #[test]
    fn cjk_short_string_unchanged() {
        assert_eq!(truncate_str("日本語", 10), "日本語");
    }

    #[test]
    fn cjk_truncated_at_char_boundary() {
        // This was the original bug — byte-index slicing panicked on CJK chars.
        // "bff側でwebsocketエラーが頻発している問題を修正" is 28 chars
        let msg = "bff側でwebsocketエラーが頻発している問題を修正";
        let result = truncate_str(msg, 15);
        // 15 - 3 = 12 chars kept + "..."
        assert_eq!(result.chars().count(), 15);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn emoji_truncated_at_char_boundary() {
        let msg = "🎉🚀✨ feat: add new feature with celebration";
        let result = truncate_str(msg, 10);
        // 10 - 3 = 7 chars kept + "..."
        assert_eq!(result.chars().count(), 10);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn mixed_cjk_ascii_truncation() {
        // Reproduces the exact scenario that caused the original panic:
        // byte-index slicing at 37 landed inside '頻' (bytes 36..39)
        let msg = ":bug: bff側でwebsocketエラーが頻発している問題を修正";
        // 35 chars, truncate at 20 to force truncation
        let result = truncate_str(msg, 20);
        assert_eq!(result.chars().count(), 20);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn empty_string() {
        assert_eq!(truncate_str("", 10), "");
    }
}
