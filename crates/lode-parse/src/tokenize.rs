use lode_core::{Token, MAX_RAW_LINE_BYTES};

use crate::error::ParseError;

/// Split a raw log line into structural tokens (RFC-0003 §3.2).
///
/// # Errors
///
/// Returns [`ParseError::EmptyLine`] for blank input and [`ParseError::LineTooLong`]
/// when `raw` exceeds [`MAX_RAW_LINE_BYTES`].
pub fn tokenize(raw: &str) -> Result<Vec<Token>, ParseError> {
    if raw.is_empty() || raw.bytes().all(|b| b.is_ascii_whitespace()) {
        return Err(ParseError::EmptyLine);
    }
    if raw.len() > MAX_RAW_LINE_BYTES {
        return Err(ParseError::line_too_long(raw.len()));
    }

    let mut first_non_space = 0;
    while first_non_space < raw.len() && raw.as_bytes()[first_non_space].is_ascii_whitespace() {
        first_non_space += 1;
    }

    let tokens = if raw.as_bytes()[first_non_space] == b'{' {
        tokenize_json(raw)
    } else {
        tokenize_generic(raw)
    };

    Ok(tokens)
}

fn tokenize_json(raw: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        let b = raw.as_bytes()[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        match b {
            b'{' | b'}' | b',' | b':' => {
                tokens.push(Token::new(raw[i..=i].to_string()));
                i += 1;
            }
            b'"' => {
                let end = scan_quoted(raw, i).expect("unclosed quote in test corpus");
                tokens.push(Token::new(&raw[i..end]));
                i = end;
            }
            _ => {
                let end = scan_json_run(raw, i);
                tokens.push(Token::new(&raw[i..end]));
                i = end;
            }
        }
    }
    tokens
}

fn tokenize_generic(raw: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        let b = raw.as_bytes()[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        let end = match b {
            b'[' => scan_bracketed(raw, i).expect("unclosed bracket in test corpus"),
            b'"' => scan_quoted(raw, i).expect("unclosed quote in test corpus"),
            b'<' => scan_pri_version(raw, i).unwrap_or_else(|| scan_default_run(raw, i)),
            _ => {
                if let Some(end) = scan_iso_timestamp(raw, i) {
                    end
                } else if let Some(end) = scan_host_port(raw, i) {
                    end
                } else {
                    scan_default_run(raw, i)
                }
            }
        };

        tokens.push(Token::new(&raw[i..end]));
        i = end;
    }
    tokens
}

fn scan_quoted(raw: &str, start: usize) -> Option<usize> {
    if raw.as_bytes().get(start) != Some(&b'"') {
        return None;
    }
    let mut i = start + 1;
    while i < raw.len() {
        if raw.as_bytes()[i] == b'"' {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

fn scan_bracketed(raw: &str, start: usize) -> Option<usize> {
    if raw.as_bytes().get(start) != Some(&b'[') {
        return None;
    }
    let mut i = start + 1;
    while i < raw.len() {
        if raw.as_bytes()[i] == b']' {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

fn scan_pri_version(raw: &str, start: usize) -> Option<usize> {
    if raw.as_bytes().get(start) != Some(&b'<') {
        return None;
    }
    let mut i = start + 1;
    while i < raw.len() && raw.as_bytes()[i].is_ascii_digit() {
        i += 1;
    }
    if raw.as_bytes().get(i) != Some(&b'>') {
        return None;
    }
    i += 1;
    while i < raw.len() && raw.as_bytes()[i].is_ascii_digit() {
        i += 1;
    }
    Some(i)
}

fn scan_iso_timestamp(raw: &str, start: usize) -> Option<usize> {
    const LEN: usize = 24;
    if start + LEN > raw.len() {
        return None;
    }
    let slice = &raw[start..start + LEN];
    let b = slice.as_bytes();
    if !is_digit_at(b, 0)
        || !is_digit_at(b, 1)
        || !is_digit_at(b, 2)
        || !is_digit_at(b, 3)
        || b[4] != b'-'
        || !is_digit_at(b, 5)
        || !is_digit_at(b, 6)
        || b[7] != b'-'
        || !is_digit_at(b, 8)
        || !is_digit_at(b, 9)
        || b[10] != b'T'
        || !is_digit_at(b, 11)
        || !is_digit_at(b, 12)
        || b[13] != b':'
        || !is_digit_at(b, 14)
        || !is_digit_at(b, 15)
        || b[16] != b':'
        || !is_digit_at(b, 17)
        || !is_digit_at(b, 18)
        || b[19] != b'.'
        || !is_digit_at(b, 20)
        || !is_digit_at(b, 21)
        || !is_digit_at(b, 22)
        || b[23] != b'Z'
    {
        return None;
    }
    Some(start + LEN)
}

fn scan_host_port(raw: &str, start: usize) -> Option<usize> {
    let mut dot_count = 0;
    let mut i = start;
    let bytes = raw.as_bytes();
    while i < raw.len() {
        let b = bytes[i];
        if b.is_ascii_digit() {
            i += 1;
            continue;
        }
        if b == b'.' && dot_count < 3 {
            dot_count += 1;
            i += 1;
            continue;
        }
        if b == b':' && dot_count == 3 {
            i += 1;
            if i >= raw.len() || !bytes[i].is_ascii_digit() {
                return None;
            }
            while i < raw.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let candidate = &raw[start..i];
            return is_host_port(candidate).then_some(i);
        }
        break;
    }
    None
}

fn is_host_port(s: &str) -> bool {
    let Some((ip, port)) = s.rsplit_once(':') else {
        return false;
    };
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    is_ip(ip)
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

fn scan_json_run(raw: &str, start: usize) -> usize {
    let mut i = start;
    while i < raw.len() {
        let b = raw.as_bytes()[i];
        if b.is_ascii_whitespace() || is_json_delimiter(b) {
            break;
        }
        i += 1;
    }
    i
}

fn scan_default_run(raw: &str, start: usize) -> usize {
    let mut i = start;
    while i < raw.len() {
        let b = raw.as_bytes()[i];
        if b.is_ascii_whitespace() || is_generic_structural(b) {
            break;
        }
        i += 1;
    }
    i
}

fn is_json_delimiter(b: u8) -> bool {
    matches!(b, b'{' | b'}' | b',' | b':' | b'"')
}

fn is_generic_structural(b: u8) -> bool {
    matches!(b, b'[' | b']' | b'"' | b'{' | b'}' | b',' | b':' | b'<')
}

fn is_digit_at(bytes: &[u8], idx: usize) -> bool {
    bytes[idx].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_line() {
        assert_eq!(tokenize(""), Err(ParseError::EmptyLine));
        assert_eq!(tokenize("   "), Err(ParseError::EmptyLine));
    }

    #[test]
    fn rejects_line_too_long() {
        let raw = "x".repeat(MAX_RAW_LINE_BYTES + 1);
        assert_eq!(
            tokenize(&raw),
            Err(ParseError::LineTooLong {
                len: MAX_RAW_LINE_BYTES + 1,
                max: MAX_RAW_LINE_BYTES,
            })
        );
    }
}
