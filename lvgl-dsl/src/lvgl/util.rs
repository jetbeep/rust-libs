use alloc::vec::Vec;

/// Builds a temporary null-terminated buffer for LVGL APIs that copy text.
///
/// The returned buffer must only be passed to LVGL calls that copy the string
/// contents before this `Vec` is dropped.
pub(crate) fn to_null_terminated(text: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(text.len() + 1);
    buf.extend_from_slice(text.as_bytes());
    buf.push(0);

    buf
}
#[cfg(test)]
mod tests {
    use super::to_null_terminated;

    #[test]
    fn non_empty_string_is_null_terminated() {
        // Invariant: every input byte is copied and a NUL byte is appended.
        assert_eq!(to_null_terminated("hello"), b"hello\0");
    }

    #[test]
    fn empty_string_produces_single_nul_byte() {
        // Invariant: empty input still produces a valid one-byte NUL buffer.
        assert_eq!(to_null_terminated(""), b"\0");
    }

    #[test]
    fn length_is_input_len_plus_one() {
        // Invariant: buffer length is always input.len() + 1.
        let s = "abc";
        let buf = to_null_terminated(s);
        assert_eq!(buf.len(), s.len() + 1);
        assert_eq!(buf[buf.len() - 1], 0);
    }
}
