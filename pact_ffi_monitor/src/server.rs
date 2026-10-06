//! TCP listener that receives NDJSON events from instrumented `pact_ffi` processes.

use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::watch;

use crate::model::{Event, Store};

pub type Shared = Arc<Mutex<Store>>;

/// Starts the listener on its own threads. Returns an error if the address cannot be bound.
pub fn spawn(addr: &str, store: Shared, notify: watch::Sender<u64>) -> std::io::Result<()> {
  let listener = TcpListener::bind(addr)?;
  std::thread::Builder::new().name("monitor-accept".into()).spawn(move || {
    for conn in listener.incoming().flatten() {
      let store = store.clone();
      let notify = notify.clone();
      std::thread::spawn(move || handle(conn, store, notify));
    }
  })?;
  Ok(())
}

fn handle(conn: TcpStream, store: Shared, notify: watch::Sender<u64>) {
  store.lock().unwrap().connections += 1;
  bump(&notify);
  let mut batch = Vec::new();
  let mut pid = None;
  let mut reader = BufReader::new(conn);
  let mut line = String::new();
  loop {
    line.clear();
    match reader.read_line(&mut line) {
      Ok(0) | Err(_) => break,
      Ok(_) => {
        if let Ok(event) = serde_json::from_str::<Event>(line.trim()) {
          match &event {
            Event::Hello(h) => pid = Some(h.pid),
            Event::Call(c) => pid = pid.or(Some(c.pid)),
            Event::Error { pid: p, .. } => pid = pid.or(Some(*p)),
            Event::Dropped { .. } => {}
          }
          batch.push(event);
        }
        // Flush when the socket has no more buffered data, so bursts are applied under one lock.
        if reader.buffer().is_empty() || batch.len() >= 256 {
          let mut s = store.lock().unwrap();
          batch.drain(..).for_each(|e| s.apply(e));
          drop(s);
          bump(&notify);
        }
      }
    }
  }
  {
    let mut s = store.lock().unwrap();
    batch.drain(..).for_each(|e| s.apply(e));
    s.connections -= 1;
    if let Some(pid) = pid {
      s.end_session(pid, SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0));
    }
  }
  bump(&notify);
}

fn bump(notify: &watch::Sender<u64>) {
  notify.send_modify(|v| *v += 1);
}
