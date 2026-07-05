//! Char-class masking with composite token rules (RFC-0003 §6.1).

use lode_core::{MaskKind, MaskedTokens, Token};

use crate::error::ParseError;
use crate::tokenize::tokenize;

/// Apply built-in masks to structural tokens.
#[must_use]
pub fn mask(tokens: Vec<Token>) -> MaskedTokens {
    let mut placeholders = Vec::new();
    let masked: Vec<Token> = tokens
        .into_iter()
        .map(|tok| mask_token(tok.as_str(), &mut placeholders))
        .collect();
    MaskedTokens {
        tokens: masked,
        placeholders,
    }
}

/// Tokenize a raw line and mask variable spans.
///
/// # Errors
///
/// Same as [`tokenize`].
pub fn tokenize_and_mask(raw: &str) -> Result<MaskedTokens, ParseError> {
    let tokens = tokenize(raw)?;
    Ok(mask(tokens))
}

fn mask_token(raw: &str, placeholders: &mut Vec<(Box<str>, Box<str>)>) -> Token {
    if let Some(masked) = mask_pri_ver(raw, placeholders) {
        return Token::new(masked);
    }
    if let Some(masked) = mask_host_port(raw, placeholders) {
        return Token::new(masked);
    }
    if let Some(masked) = mask_quoted_http(raw, placeholders) {
        return Token::new(masked);
    }
    if let Some(masked) = mask_quoted_uuid_or_hex(raw, placeholders) {
        return Token::new(masked);
    }

    if let Some((kind, original)) = first_plain_mask(raw) {
        record_capture(placeholders, kind, original);
        return Token::new(kind.placeholder());
    }

    Token::new(raw)
}

fn mask_pri_ver(raw: &str, placeholders: &mut Vec<(Box<str>, Box<str>)>) -> Option<String> {
    let bytes = raw.as_bytes();
    if bytes.first() != Some(&b'<') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if bytes.get(i) != Some(&b'>') {
        return None;
    }
    let pri_end = i + 1;
    if pri_end >= raw.len() || !bytes[pri_end..].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let pri = &raw[..pri_end];
    record_capture(placeholders, MaskKind::Num, pri);
    Some(format!(
        "{}{}",
        MaskKind::Num.placeholder(),
        &raw[pri_end..]
    ))
}

fn mask_host_port(raw: &str, placeholders: &mut Vec<(Box<str>, Box<str>)>) -> Option<String> {
    if !is_host_port(raw) {
        return None;
    }
    let (ip, port) = raw.rsplit_once(':')?;
    record_capture(placeholders, MaskKind::Ip, ip);
    record_capture(placeholders, MaskKind::Num, port);
    Some(format!(
        "{}:{}",
        MaskKind::Ip.placeholder(),
        MaskKind::Num.placeholder()
    ))
}

fn mask_quoted_http(raw: &str, placeholders: &mut Vec<(Box<str>, Box<str>)>) -> Option<String> {
    let inner = strip_quotes(raw)?;
    let (method, rest) = inner.split_once(' ')?;
    if method.is_empty() || !method.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    // Match `" HTTP/"` and parse the full version (`major[.minor]`) so HTTP/2 and
    // HTTP/3 are recognised (not just HTTP/1.x), and preserve anything after the
    // version — masking must never delete request-line content.
    let (path, after) = rest.rsplit_once(" HTTP/")?;
    if path.is_empty() || !path.starts_with('/') {
        return None;
    }
    let version_len = http_version_len(after)?;
    let (version, trailing) = after.split_at(version_len);
    record_capture(placeholders, MaskKind::Path, path);
    Some(format!(
        "\"{method} {} HTTP/{version}{trailing}\"",
        MaskKind::Path.placeholder()
    ))
}

/// Byte length of a leading HTTP version token (`major` or `major.minor`) in `s`,
/// or `None` if `s` does not start with a version.
fn http_version_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None; // no major-version digit
    }
    if i < b.len() && b[i] == b'.' {
        let minor_start = i + 1;
        i = minor_start;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == minor_start {
            return None; // `.` not followed by a minor-version digit
        }
    }
    Some(i)
}

fn mask_quoted_uuid_or_hex(
    raw: &str,
    placeholders: &mut Vec<(Box<str>, Box<str>)>,
) -> Option<String> {
    let inner = strip_quotes(raw)?;
    if is_uuid(inner) {
        record_capture(placeholders, MaskKind::Uuid, inner);
        return Some(format!("\"{}\"", MaskKind::Uuid.placeholder()));
    }
    if is_hex(inner) {
        record_capture(placeholders, MaskKind::Hex, inner);
        return Some(format!("\"{}\"", MaskKind::Hex.placeholder()));
    }
    None
}

fn first_plain_mask(raw: &str) -> Option<(MaskKind, &str)> {
    for kind in MaskKind::EVAL_ORDER {
        if matches_kind(kind, raw) {
            return Some((kind, raw));
        }
    }
    None
}

fn matches_kind(kind: MaskKind, raw: &str) -> bool {
    match kind {
        MaskKind::Ts => is_ts_bracket(raw) || is_ts_iso(raw),
        MaskKind::Uuid => is_uuid(raw),
        MaskKind::Ip => is_ip(raw),
        MaskKind::Url => is_url(raw),
        MaskKind::Email => is_email(raw),
        MaskKind::Path => is_path(raw),
        MaskKind::Hex => is_hex(raw),
        MaskKind::Num => is_num(raw),
        MaskKind::Quoted | MaskKind::Wildcard => false,
    }
}

fn record_capture(placeholders: &mut Vec<(Box<str>, Box<str>)>, kind: MaskKind, original: &str) {
    placeholders.push((
        kind.placeholder().into(),
        original.to_string().into_boxed_str(),
    ));
}

fn strip_quotes(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'"' || bytes[bytes.len() - 1] != b'"' {
        return None;
    }
    Some(&raw[1..raw.len() - 1])
}

fn is_num(s: &str) -> bool {
    let t = s.trim_start_matches('-');
    !t.is_empty()
        && t.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && t.bytes().any(|b| b.is_ascii_digit())
}

fn is_ip(s: &str) -> bool {
    let mut octets = 0;
    for part in s.split('.') {
        if part.is_empty() || part.len() > 3 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        let Ok(value) = part.parse::<u16>() else {
            return false;
        };
        if value > 255 {
            return false;
        }
        octets += 1;
    }
    octets == 4
}

fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.as_bytes().iter().enumerate().all(|(i, &b)| {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

fn is_hex(s: &str) -> bool {
    // Require at least one `a-f`/`A-F`: `is_ascii_hexdigit` also accepts `0-9`, so a
    // pure decimal of 8+ digits (epoch `1728568536`, byte count `12345678`) would
    // otherwise match here and mask as `<HEX>` — but `Hex` precedes `Num` in
    // `EVAL_ORDER`, so it would win and produce the wrong template.
    s.len() >= 8
        && s.bytes().all(|b| b.is_ascii_hexdigit())
        && s.bytes().any(|b| b.is_ascii_alphabetic())
}

fn is_path(s: &str) -> bool {
    s.contains('/') && s.len() > 1
}

fn is_ts_bracket(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 4
        && bytes[0] == b'['
        && bytes[bytes.len() - 1] == b']'
        && s.matches(':').count() >= 2
        && s.bytes().any(|b| b.is_ascii_digit())
}

fn is_ts_iso(s: &str) -> bool {
    const LEN: usize = 24;
    if s.len() != LEN {
        return false;
    }
    let b = s.as_bytes();
    digit_at(b, 0)
        && digit_at(b, 1)
        && digit_at(b, 2)
        && digit_at(b, 3)
        && b[4] == b'-'
        && digit_at(b, 5)
        && digit_at(b, 6)
        && b[7] == b'-'
        && digit_at(b, 8)
        && digit_at(b, 9)
        && b[10] == b'T'
        && digit_at(b, 11)
        && digit_at(b, 12)
        && b[13] == b':'
        && digit_at(b, 14)
        && digit_at(b, 15)
        && b[16] == b':'
        && digit_at(b, 17)
        && digit_at(b, 18)
        && b[19] == b'.'
        && digit_at(b, 20)
        && digit_at(b, 21)
        && digit_at(b, 22)
        && b[23] == b'Z'
}

fn is_host_port(s: &str) -> bool {
    let Some((ip, port)) = s.rsplit_once(':') else {
        return false;
    };
    !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) && is_ip(ip)
}

fn is_url(_s: &str) -> bool {
    false
}

fn is_email(_s: &str) -> bool {
    false
}

fn digit_at(bytes: &[u8], idx: usize) -> bool {
    bytes[idx].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lode_core::Token;

    #[test]
    fn mask_records_placeholders() {
        let tokens = vec![Token::new("200")];
        let masked = mask(tokens);
        assert_eq!(masked.tokens[0].as_str(), "<NUM>");
        assert_eq!(masked.placeholders.len(), 1);
        assert_eq!(masked.placeholders[0].0.as_ref(), "<NUM>");
        assert_eq!(masked.placeholders[0].1.as_ref(), "200");
    }

    #[test]
    fn pri_version_partial_replace() {
        let tokens = vec![Token::new("<134>1")];
        let masked = mask(tokens);
        assert_eq!(masked.tokens[0].as_str(), "<NUM>1");
    }

    #[test]
    fn long_decimal_masks_as_num_not_hex() {
        // Epoch / byte-count fields (8+ digits) must be <NUM>, not <HEX>.
        for decimal in ["1728568536", "12345678", "00000000"] {
            let masked = mask(vec![Token::new(decimal)]);
            assert_eq!(masked.tokens[0].as_str(), "<NUM>", "input: {decimal}");
        }
    }

    #[test]
    fn real_hex_still_masks_as_hex() {
        for hex in ["deadbeef", "00ff1a2b", "DEADBEEF"] {
            let masked = mask(vec![Token::new(hex)]);
            assert_eq!(masked.tokens[0].as_str(), "<HEX>", "input: {hex}");
        }
    }

    #[test]
    fn http2_request_line_keeps_method_and_version() {
        let masked = mask(vec![Token::new("\"GET /x HTTP/2\"")]);
        assert_eq!(masked.tokens[0].as_str(), "\"GET <PATH> HTTP/2\"");
    }

    #[test]
    fn http_mask_preserves_trailing_content() {
        let masked = mask(vec![Token::new("\"GET /a/b HTTP/1.0 keepalive\"")]);
        assert_eq!(
            masked.tokens[0].as_str(),
            "\"GET <PATH> HTTP/1.0 keepalive\""
        );
    }

    #[test]
    fn http11_request_line_unchanged() {
        let masked = mask(vec![Token::new("\"GET /api/users/12 HTTP/1.1\"")]);
        assert_eq!(masked.tokens[0].as_str(), "\"GET <PATH> HTTP/1.1\"");
    }
}
