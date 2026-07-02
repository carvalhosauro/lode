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
    Some(format!("{}{}", MaskKind::Num.placeholder(), &raw[pri_end..]))
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
    let (path, suffix) = rest.rsplit_once(" HTTP/1.")?;
    if path.is_empty() || !path.starts_with('/') {
        return None;
    }
    if !method.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let version = suffix
        .chars()
        .next()
        .filter(char::is_ascii_digit)?;
    record_capture(placeholders, MaskKind::Path, path);
    Some(format!(
        "\"{method} {} HTTP/1.{version}\"",
        MaskKind::Path.placeholder()
    ))
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
    s.len() >= 8 && s.bytes().all(|b| b.is_ascii_hexdigit())
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
    !port.is_empty()
        && port.bytes().all(|b| b.is_ascii_digit())
        && is_ip(ip)
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
}
