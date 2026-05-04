//! Tiny JSONL line iterator.
//!
//! Wraps a `BufRead` and yields one JSON `Map` per non-empty line. Tracks
//! the line number so parse errors can carry a useful location.

use crate::error::{Result, TraceError};
use serde_json::{Map, Value};
use std::io::BufRead;

pub(crate) struct JsonLines<R: BufRead> {
    reader: R,
    line: usize,
    buf: String,
}

impl<R: BufRead> JsonLines<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line: 0,
            buf: String::new(),
        }
    }
}

impl<R: BufRead> Iterator for JsonLines<R> {
    type Item = Result<Map<String, Value>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.buf.clear();
            self.line += 1;
            match self.reader.read_line(&mut self.buf) {
                Ok(0) => return None,
                Ok(_) => {}
                Err(e) => return Some(Err(TraceError::Io(e))),
            }
            // Trim trailing newline; skip blank lines silently.
            let trimmed = self.buf.trim_end_matches(['\n', '\r']);
            if trimmed.trim().is_empty() {
                continue;
            }
            let line = self.line;
            let parsed: serde_json::Result<Value> = serde_json::from_str(trimmed);
            return match parsed {
                Ok(Value::Object(map)) => Some(Ok(map)),
                Ok(_) => Some(Err(TraceError::Json {
                    line,
                    source: serde::de::Error::custom("expected JSON object"),
                })),
                Err(source) => Some(Err(TraceError::Json { line, source })),
            };
        }
    }
}
