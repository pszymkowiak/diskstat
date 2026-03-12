/// Shared utility functions for the application.
use std::fmt::Write;

/// Format byte size into a reusable buffer (optimized to avoid allocation).
pub fn format_size_into(bytes: u64, buf: &mut String) {
    buf.clear();
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        let _ = write!(buf, "{:.1}G", bytes as f64 / GB as f64);
    } else if bytes >= MB {
        let _ = write!(buf, "{:.1}M", bytes as f64 / MB as f64);
    } else if bytes >= KB {
        let _ = write!(buf, "{}K", bytes / KB);
    } else {
        let _ = write!(buf, "{}B", bytes);
    }
}

/// Format byte size into a new String (convenience wrapper).
pub fn format_size(bytes: u64) -> String {
    let mut buf = String::with_capacity(16);
    format_size_into(bytes, &mut buf);
    buf
}

/// Safely truncate a string to at most `max_chars` characters without panicking on multi-byte chars.
pub fn truncate_str(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// Format file age (mtime) as a relative time string.
pub fn format_age(mtime: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if mtime == 0 || mtime > now {
        return String::new();
    }
    let age_secs = now - mtime;
    let days = age_secs / 86400;
    if days == 0 {
        "<1d".to_string()
    } else if days < 14 {
        format!("{}d", days)
    } else if days < 60 {
        format!("{}w", days / 7)
    } else if days < 365 {
        format!("{}m", days / 30)
    } else {
        format!("{}y", days / 365)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(512), "512B");
        assert_eq!(format_size(1024), "1K");
        assert_eq!(format_size(1536), "1K");
        assert_eq!(format_size(1048576), "1.0M");
        assert_eq!(format_size(1073741824), "1.0G");
    }

    #[test]
    fn test_truncate_str_ascii() {
        assert_eq!(truncate_str("hello", 3), "hel");
        assert_eq!(truncate_str("hi", 10), "hi");
        assert_eq!(truncate_str("", 5), "");
    }

    #[test]
    fn test_truncate_str_unicode() {
        // Multi-byte chars should not panic
        assert_eq!(truncate_str("héllo", 3), "hél");
        assert_eq!(truncate_str("你好世界", 2), "你好");
        assert_eq!(truncate_str("🔥fire", 2), "🔥f");
    }

    #[test]
    fn test_format_age_zero() {
        assert_eq!(format_age(0), "");
    }

    #[test]
    fn test_format_age_future() {
        let future = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            + 1000;
        assert_eq!(format_age(future), "");
    }

    #[test]
    fn test_format_age_less_than_day() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let recent = now - 3600; // 1 hour ago
        assert_eq!(format_age(recent), "<1d");
    }

    #[test]
    fn test_format_age_days() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let days_ago = now - (5 * 86400); // 5 days ago
        assert_eq!(format_age(days_ago), "5d");
    }

    #[test]
    fn test_format_age_weeks() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let weeks_ago = now - (21 * 86400); // 21 days ago
        assert_eq!(format_age(weeks_ago), "3w");
    }

    #[test]
    fn test_format_age_months() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let months_ago = now - (90 * 86400); // 90 days ago
        assert_eq!(format_age(months_ago), "3m");
    }

    #[test]
    fn test_format_age_years() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let years_ago = now - (730 * 86400); // 730 days ago
        assert_eq!(format_age(years_ago), "2y");
    }

    #[test]
    fn test_truncate_str_empty() {
        assert_eq!(truncate_str("", 0), "");
        assert_eq!(truncate_str("", 10), "");
    }

    #[test]
    fn test_truncate_str_exact_length() {
        assert_eq!(truncate_str("hello", 5), "hello");
        assert_eq!(truncate_str("test", 4), "test");
    }
}
