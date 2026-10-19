/// Quote shell text as one literal word, preserving its characters exactly.
pub fn quote(text: &str, double: bool, always: bool) -> String {
    if !always && !text.is_empty() && text.chars().all(is_safe_unquoted) {
        return text.to_owned();
    }

    if double {
        let mut quoted = String::from("\"");
        for ch in text.chars() {
            if matches!(ch, '$' | '`' | '"' | '\\') {
                quoted.push('\\');
            }
            quoted.push(ch);
        }
        quoted.push('"');
        quoted
    } else {
        let mut quoted = String::from("'");
        for ch in text.chars() {
            if ch == '\'' {
                quoted.push_str("'\\''");
            } else {
                quoted.push(ch);
            }
        }
        quoted.push('\'');
        quoted
    }
}

fn is_safe_unquoted(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            '_' | '@' | '%' | '+' | '=' | ':' | ',' | '.' | '/' | '-'
        )
}
