//! Bounded, credential-redacting text log shared by restore, tool installation and LSP.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};

pub const MAX_LOG_LINES: usize = 400;
pub const MAX_LOG_CHARS: usize = 60_000;
const MAX_LINE_CHARS: usize = 4_000;

#[derive(Default)]
pub struct LogBuffer {
    lines: VecDeque<String>,
    chars: usize,
    dropped: usize,
}

impl LogBuffer {
    pub fn push(&mut self, line: String) {
        let mut line = redact_url_credentials(&line);
        if line.chars().count() > MAX_LINE_CHARS {
            line = format!("{}…", line.chars().take(MAX_LINE_CHARS).collect::<String>());
        }
        self.chars += line.chars().count() + 1;
        self.lines.push_back(line);
        while self.lines.len() > MAX_LOG_LINES || self.chars > MAX_LOG_CHARS {
            let Some(front) = self.lines.pop_front() else {
                break;
            };
            self.chars = self.chars.saturating_sub(front.chars().count() + 1);
            self.dropped += 1;
        }
    }

    pub fn text(&self) -> (String, bool) {
        let mut text = String::new();
        if self.dropped > 0 {
            text.push_str(&format!(
                "… log truncated: {} earlier line(s) not kept; showing the last {} line(s) …\n",
                self.dropped,
                self.lines.len()
            ));
        }
        for line in &self.lines {
            text.push_str(line);
            text.push('\n');
        }
        (text.trim_end().to_string(), self.dropped > 0)
    }
}

/// Streams one pipe line by line. The caller runs this on its own thread so a full pipe can
/// never block the child.
pub fn pump_lines(reader: impl Read, stderr: bool, mut on_line: impl FnMut(&str, bool)) {
    let mut reader = BufReader::new(reader);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let text = String::from_utf8_lossy(&bytes);
        let text = text.trim_end_matches(['\r', '\n']);
        if !text.is_empty() {
            on_line(text, stderr);
        }
    }
}

/// Feed URLs can carry credentials; the visible log never keeps the userinfo part.
pub fn redact_url_credentials(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(index) = rest.find("://") {
        let (head, tail) = rest.split_at(index + 3);
        out.push_str(head);
        let end = tail
            .find(['/', ' ', '"', '\'', ')'])
            .unwrap_or(tail.len());
        match tail[..end].find('@') {
            Some(at) => {
                out.push_str("***@");
                rest = &tail[at + 1..];
            }
            None => rest = tail,
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_credentials_are_not_kept_in_the_log() {
        let line = "error: source https://user:secret@feed.example/v3/index.json failed";
        assert_eq!(
            redact_url_credentials(line),
            "error: source https://***@feed.example/v3/index.json failed"
        );
        assert_eq!(
            redact_url_credentials("https://feed.example/v3/index.json"),
            "https://feed.example/v3/index.json"
        );
    }

    #[test]
    fn the_log_keeps_the_tail_and_reports_truncation() {
        let mut log = LogBuffer::default();
        for index in 0..(MAX_LOG_LINES + 5) {
            log.push(format!("line {index}"));
        }
        let (text, truncated) = log.text();
        assert!(truncated);
        assert!(text.starts_with("… log truncated: 5 earlier line(s) not kept"));
        assert!(text.ends_with(&format!("line {}", MAX_LOG_LINES + 4)));
        assert!(!text.contains("line 0\n"));
    }
}
