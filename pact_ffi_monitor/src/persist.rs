//! Saving the captured events to an NDJSON file and loading them back.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::model::{Event, Store};

/// Writes every session and retained call to `path`, one JSON event per line. Returns the line count.
pub fn export(path: &str, store: &Store) -> std::io::Result<usize> {
  let events = store.export();
  let mut out = BufWriter::new(File::create(path)?);
  for event in &events {
    serde_json::to_writer(&mut out, event)?;
    out.write_all(b"\n")?;
  }
  out.flush()?;
  Ok(events.len())
}

/// Applies the events in `path` to the store. Lines that do not parse are skipped.
/// Returns the number of events applied.
pub fn import(path: &str, store: &mut Store) -> std::io::Result<usize> {
  let mut applied = 0;
  for line in BufReader::new(File::open(path)?).lines() {
    if let Ok(event) = serde_json::from_str::<Event>(line?.trim()) {
      store.apply(event);
      applied += 1;
    }
  }
  Ok(applied)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn round_trips_through_a_file() {
    let mut s = Store::default();
    s.apply(serde_json::from_value(serde_json::json!({
      "type": "call", "seq": 0, "pid": 1, "ts_ms": 1, "duration_us": 1, "thread": "t",
      "function": "f", "module": "m", "args": [], "result": "true", "panicked": false
    })).unwrap());
    let path = std::env::temp_dir().join(format!("pact_ffi_monitor_test_{}.ndjson", std::process::id()));
    let path = path.to_string_lossy().to_string();
    assert_eq!(export(&path, &s).unwrap(), 1);
    let mut t = Store::default();
    assert_eq!(import(&path, &mut t).unwrap(), 1);
    assert_eq!(t.calls.len(), 1);
    let _ = std::fs::remove_file(path);
  }
}
