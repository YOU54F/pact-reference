//! Runs in its own process so the monitor env var is read before any FFI call.

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::time::Duration;

use pact_ffi::mock_server::handles::pactffi_new_pact;
use pact_ffi::monitor::MONITOR_ENV_VAR;
use serde_json::Value;

#[test]
fn reports_ffi_calls_to_monitor() {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let addr = listener.local_addr().unwrap();
  unsafe { std::env::set_var(MONITOR_ENV_VAR, addr.to_string()) };

  let consumer = std::ffi::CString::new("c").unwrap();
  let provider = std::ffi::CString::new("p").unwrap();
  pactffi_new_pact(consumer.as_ptr(), provider.as_ptr());

  let (conn, _) = listener.accept().unwrap();
  conn.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
  let mut lines = BufReader::new(conn).lines();
  let hello: Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
  assert_eq!(hello["type"], "hello");
  let call: Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
  assert_eq!(call["type"], "call");
  assert_eq!(call["function"], "pactffi_new_pact");
  assert_eq!(call["args"], serde_json::json!([["consumer_name", "\"c\""], ["provider_name", "\"p\""]]));
  assert_eq!(call["panicked"], false);
}
