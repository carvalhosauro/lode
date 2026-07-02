use lode_parse::tokenize;

#[test]
fn nginx_bracket_and_quoted_tokens() {
    let raw = r#"127.0.0.1 - - [10/Oct/2024:13:55:36 +0000] "GET /api/users/12 HTTP/1.1" 200 1500"#;
    let tokens: Vec<_> = tokenize(raw)
        .unwrap()
        .into_iter()
        .map(|t| t.as_str().to_string())
        .collect();
    assert_eq!(tokens[0], "127.0.0.1");
    assert_eq!(tokens[3], "[10/Oct/2024:13:55:36 +0000]");
    assert_eq!(tokens[4], r#""GET /api/users/12 HTTP/1.1""#);
    assert_eq!(
        tokens,
        vec![
            "127.0.0.1",
            "-",
            "-",
            "[10/Oct/2024:13:55:36 +0000]",
            r#""GET /api/users/12 HTTP/1.1""#,
            "200",
            "1500",
        ]
    );
}

#[test]
fn syslog_pri_version_iso_hostport() {
    let raw = "<134>1 2024-10-10T13:55:36.123Z host01 myapp - - - User login successful for user 100";
    let tokens: Vec<_> = tokenize(raw)
        .unwrap()
        .into_iter()
        .map(|t| t.as_str().to_string())
        .collect();
    assert_eq!(tokens[0], "<134>1");
    assert_eq!(tokens[1], "2024-10-10T13:55:36.123Z");
    assert_eq!(
        tokens,
        vec![
            "<134>1",
            "2024-10-10T13:55:36.123Z",
            "host01",
            "myapp",
            "-",
            "-",
            "-",
            "User",
            "login",
            "successful",
            "for",
            "user",
            "100",
        ]
    );

    let hostport =
        "<131>1 2024-10-10T13:56:37.123Z host01 myapp - - - Connection refused to 10.0.1.1:8001";
    let hostport_tokens: Vec<_> = tokenize(hostport)
        .unwrap()
        .into_iter()
        .map(|t| t.as_str().to_string())
        .collect();
    assert_eq!(hostport_tokens.last().map(String::as_str), Some("10.0.1.1:8001"));
}

#[test]
fn json_preserves_delimiters_and_spaces() {
    let raw = r#"{ "level" : "info" , "msg" : "user login" , "user_id" : 100 }"#;
    let tokens: Vec<_> = tokenize(raw)
        .unwrap()
        .into_iter()
        .map(|t| t.as_str().to_string())
        .collect();
    assert_eq!(
        tokens,
        vec![
            "{",
            "\"level\"",
            ":",
            "\"info\"",
            ",",
            "\"msg\"",
            ":",
            "\"user login\"",
            ",",
            "\"user_id\"",
            ":",
            "100",
            "}",
        ]
    );
}
