//! Events produced before a monitor is listening are replayed once it appears.

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::time::Duration;

use pact_ffi::mock_server::handles::pactffi_new_pact;
use pact_ffi::monitor::MONITOR_ENV_VAR;
use serde_json::Value;

#[test]
fn replays_calls_made_before_the_monitor_started() {
  // Reserve an address, then free it so the first connection attempts are refused.
  let addr = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
  unsafe { std::env::set_var(MONITOR_ENV_VAR, addr.to_string()) };

  let (consumer, provider) = (std::ffi::CString::new("c").unwrap(), std::ffi::CString::new("p").unwrap());
  pactffi_new_pact(consumer.as_ptr(), provider.as_ptr());
  std::thread::sleep(Duration::from_millis(300));

  let listener = TcpListener::bind(addr).unwrap();
  let (conn, _) = listener.accept().unwrap();
  conn.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
  let mut lines = BufReader::new(conn).lines();
  let hello: Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
  assert_eq!(hello["type"], "hello");
  let call: Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
  assert_eq!(call["function"], "pactffi_new_pact");
}
