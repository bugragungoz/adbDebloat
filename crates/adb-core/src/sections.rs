//! Several shell commands in one `adb shell` call, each output under a marker line, so a phone is
//! asked once instead of five times.

use std::collections::HashMap;

/// The line that starts a section in the output.
pub fn marker(name: &str) -> String {
    format!("@@adbdebloat:{name}@@")
}

/// `echo <marker>; <command>` for each section, joined into one shell line.
pub fn script(parts: &[(&str, &str)]) -> String {
    parts
        .iter()
        .map(|(name, command)| format!("echo {}; {command}", marker(name)))
        .collect::<Vec<_>>()
        .join("; ")
}

/// The text of each section by name. Text before the first marker is dropped.
pub fn split_sections(text: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current: Option<String> = None;
    let mut body = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("@@adbdebloat:").and_then(|r| r.strip_suffix("@@")) {
            if let Some(done) = current.take() {
                sections.insert(done, std::mem::take(&mut body));
            }
            current = Some(name.to_string());
            body.clear();
        } else if current.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(done) = current {
        sections.insert(done, body);
    }
    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_splits_sections() {
        assert_eq!(
            script(&[("a", "getprop"), ("b", "pm list packages")]),
            "echo @@adbdebloat:a@@; getprop; echo @@adbdebloat:b@@; pm list packages"
        );
        let text = "noise\n@@adbdebloat:a@@\none\ntwo\n@@adbdebloat:b@@\r\n@@adbdebloat:c@@\nthree";
        let s = split_sections(text);
        assert_eq!(s["a"], "one\ntwo\n");
        assert_eq!(s["b"], "");
        assert_eq!(s["c"], "three\n");
        assert_eq!(s.len(), 3);
    }
}
