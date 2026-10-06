mod model;
mod persist;
mod rows;
mod server;
mod time;
mod ui;

use std::sync::{Arc, Mutex};

use freya::prelude::*;
use tokio::sync::watch;

use crate::model::Store;

const DEFAULT_ADDR: &str = "127.0.0.1:7777";

fn main() {
  // Usage: pact_ffi_monitor [ADDR] [--import FILE]
  let mut addr = None;
  let mut import = None;
  let mut args = std::env::args().skip(1);
  while let Some(arg) = args.next() {
    match arg.as_str() {
      "--import" => import = args.next(),
      _ => addr = Some(arg),
    }
  }
  let addr = addr.or_else(|| std::env::var("PACT_FFI_MONITOR").ok()).unwrap_or_else(|| DEFAULT_ADDR.to_string());

  let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
  let _guard = rt.enter();

  let store = Arc::new(Mutex::new(Store::default()));
  if let Some(path) = import {
    match persist::import(&path, &mut store.lock().unwrap()) {
      Ok(n) => eprintln!("Imported {} events from {}", n, path),
      Err(e) => eprintln!("Could not import {}: {}", path, e),
    }
  }
  let (tx, rx) = watch::channel(0u64);
  let listen_error = server::spawn(&addr, store.clone(), tx.clone()).err().map(|e| e.to_string());

  launch(LaunchConfig::new().with_window(
    WindowConfig::new_app(ui::MonitorApp { store, rx, addr, listen_error, notify: tx })
      .with_title("Pact FFI Monitor")
      .with_size(1400.0, 800.0)
  ))
}
