//! Mask-only corpus gate — masked patterns must match golden templates (T1.1b).

mod common;

use common::corpus_loader::{corpus_root, load_corpus};
use lode_core::patterns_match;
use lode_parse::{pattern_to_string, tokenize_and_mask};

fn mask_pattern(raw: &str) -> String {
    let masked = tokenize_and_mask(raw).unwrap();
    pattern_to_string(&masked.tokens)
}

#[test]
fn mask_only_pa_nginx_sample() {
    let raw = r#"127.0.0.1 - - [10/Oct/2024:13:55:36 +0000] "GET /api/users/12 HTTP/1.1" 200 1500"#;
    let exp = r#"<IP> - - <TS> "GET <PATH> HTTP/1.1" <NUM> <NUM>"#;
    assert_eq!(mask_pattern(raw), exp);
}

#[test]
fn mask_only_pa_syslog_pri_and_hostport() {
    let login = "<134>1 2024-10-10T13:55:36.123Z host01 myapp - - - User login successful for user 100";
    assert_eq!(
        mask_pattern(login),
        "<NUM>1 <TS> host01 myapp - - - User login successful for user <NUM>"
    );

    let refused =
        "<131>1 2024-10-10T13:56:37.123Z host01 myapp - - - Connection refused to 10.0.1.1:8001";
    assert_eq!(
        mask_pattern(refused),
        "<NUM>1 <TS> host01 myapp - - - Connection refused to <IP>:<NUM>"
    );
}

#[test]
fn mask_only_pa_json_quoted_hex_and_uuid() {
    let warn = r#"{ "level" : "warn" , "msg" : "cache miss" , "key" : "deadbeefcafebabe" }"#;
    assert_eq!(
        mask_pattern(warn),
        r#"{ "level" : "warn" , "msg" : "cache miss" , "key" : "<HEX>" }"#
    );

    let err = r#"{ "level" : "error" , "msg" : "db connection failed" , "duration_ms" : 3002 , "request_id" : "7c9e6679-7425-40de-944b-e07fc1f90ae7" }"#;
    assert_eq!(
        mask_pattern(err),
        r#"{ "level" : "error" , "msg" : "db connection failed" , "duration_ms" : <NUM> , "request_id" : "<UUID>" }"#
    );
}

#[test]
fn mask_only_pa_full_corpus() {
    let input = load_corpus(corpus_root());
    let mut correct = 0usize;
    let mut total = 0usize;
    let mut failures = Vec::new();

    for format in &input.formats {
        for (idx, raw) in format.lines.iter().enumerate() {
            let line_no = idx + 1;
            let label = format.labels.get(&line_no).expect("label");
            let expected = format
                .templates
                .get(&label.template_gid)
                .expect("template");
            let mined = mask_pattern(raw);
            total += 1;
            if patterns_match(expected, &mined) {
                correct += 1;
            } else {
                failures.push(format!(
                    "{} line {line_no}: expected {expected:?}, got {mined:?}",
                    format.spec.id
                ));
            }
        }
    }

    assert_eq!(total, 165, "corpus must have 165 lines");
    assert!(
        failures.is_empty(),
        "mask-only PA {}/{}:\n{}",
        correct,
        total,
        failures.join("\n")
    );
    assert_eq!(correct, 165);
}
