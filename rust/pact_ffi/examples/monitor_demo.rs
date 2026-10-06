//! Generates FFI traffic from several threads, for trying out the `pact_ffi_monitor` GUI:
//!
//! ```text
//! PACT_FFI_MONITOR=127.0.0.1:7777 cargo run --example monitor_demo
//! ```

use std::ffi::CString;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::ptr::null;
use std::time::Duration;

use pact_ffi::mock_server::handles::{
  pactffi_given, pactffi_new_interaction, pactffi_new_pact, pactffi_pact_handle_write_file,
  pactffi_response_status, pactffi_upon_receiving, pactffi_with_request
};
use pact_ffi::mock_server::{pactffi_cleanup_mock_server, pactffi_create_mock_server_for_transport, pactffi_mock_server_matched};

fn c(s: &str) -> CString {
  CString::new(s).unwrap()
}

fn run(index: usize) {
  let (consumer, provider) = (c(&format!("consumer-{}", index)), c("provider"));
  let pact = pactffi_new_pact(consumer.as_ptr(), provider.as_ptr());
  for i in 0..2 {
    let interaction = pactffi_new_interaction(pact, c(&format!("request {}", i)).as_ptr());
    pactffi_given(interaction, c("a state").as_ptr());
    pactffi_upon_receiving(interaction, c("a request").as_ptr());
    pactffi_with_request(interaction, c("GET").as_ptr(), c(&format!("/thing/{}", i)).as_ptr());
    pactffi_response_status(interaction, 200);
  }
  let port = pactffi_create_mock_server_for_transport(pact, c("127.0.0.1").as_ptr(), 0, c("http").as_ptr(), null());
  if port > 0 {
    // Even numbered consumers make the requests; odd ones don't, so their mock server reports a failure.
    if index % 2 == 0 {
      for i in 0..2 {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port as u16)) {
          let _ = write!(stream, "GET /thing/{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", i);
          let mut response = String::new();
          let _ = stream.read_to_string(&mut response);
        }
      }
    }
    std::thread::sleep(Duration::from_millis(20 * (index as u64 + 1)));
    pactffi_mock_server_matched(port);
    let dir = std::env::temp_dir();
    let dir = c(&dir.to_string_lossy());
    pactffi_pact_handle_write_file(pact, dir.as_ptr(), true);
    pactffi_cleanup_mock_server(port);
  }
}

fn main() {
  let threads: Vec<_> = (0..4).map(|i| {
    std::thread::Builder::new().name(format!("consumer-{}", i)).spawn(move || run(i)).unwrap()
  }).collect();
  threads.into_iter().for_each(|t| t.join().unwrap());
  // Give the monitor thread a moment to flush.
  std::thread::sleep(Duration::from_millis(500));
}
