use super::{format_message_datetime, wrap_message_lines, wrap_plain_lines};

#[test]
fn format_message_datetime_parses_iso_timestamp() {
    let raw = "2026-02-12T13:44:59Z";
    let formatted = format_message_datetime(raw);
    assert_eq!(formatted.as_deref(), Some("12/02/2026 13:44"));
}

#[test]
fn format_message_datetime_handles_empty_and_unknown_formats() {
    assert_eq!(format_message_datetime(""), None);
    assert_eq!(
        format_message_datetime("not-a-date").as_deref(),
        Some("not-a-date")
    );
}

#[test]
fn wrap_plain_lines_splits_fixed_width_and_preserves_newlines() {
    let wrapped = wrap_plain_lines("ab\ncdef", 3);
    assert_eq!(wrapped, vec!["ab", "cde", "f"]);
}

#[test]
fn wrap_message_lines_indents_following_lines() {
    let wrapped = wrap_message_lines("Rafael: ", "1234567890", 12);
    assert_eq!(wrapped, vec!["Rafael: 1234", "        5678", "        90"]);
}
