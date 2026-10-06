//! Timestamp formatting, in local time by default (UTC on request, or where the offset is unknown).

use std::sync::atomic::{AtomicBool, Ordering};

static UTC: AtomicBool = AtomicBool::new(false);

pub fn is_utc() -> bool {
  UTC.load(Ordering::Relaxed)
}

pub fn toggle_utc() {
  UTC.store(!is_utc(), Ordering::Relaxed);
}

/// Offset of local time from UTC, in seconds, at the given unix time.
#[cfg(unix)]
fn local_offset_secs(unix_secs: i64) -> i64 {
  // SAFETY: localtime_r only writes to the tm we pass in.
  unsafe {
    let t: libc::time_t = unix_secs as libc::time_t;
    let mut tm: libc::tm = std::mem::zeroed();
    if libc::localtime_r(&t, &mut tm).is_null() { 0 } else { tm.tm_gmtoff as i64 }
  }
}

#[cfg(not(unix))]
fn local_offset_secs(_unix_secs: i64) -> i64 {
  0
}

/// Formats the time of day as `HH:MM:SS.mmm`.
pub fn fmt_time(ts_ms: u64) -> String {
  let secs = (ts_ms / 1000) as i64;
  let offset = if is_utc() { 0 } else { local_offset_secs(secs) };
  let s = (secs + offset).rem_euclid(86_400);
  format!("{:02}:{:02}:{:02}.{:03}", s / 3600, s / 60 % 60, s % 60, ts_ms % 1000)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn formats_utc() {
    UTC.store(true, Ordering::Relaxed);
    assert_eq!(fmt_time(3_723_004), "01:02:03.004");
    UTC.store(false, Ordering::Relaxed);
  }
}
