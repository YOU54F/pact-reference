//! Opt-in call monitor for the FFI. When the `PACT_FFI_MONITOR` environment variable is set to a
//! `host:port` address, every invocation of an FFI function declared with `ffi_fn!` is reported
//! as a line of JSON (NDJSON) over TCP to that address. This is used by the `pact_ffi_monitor`
//! companion GUI. When the variable is not set, the overhead is a single atomic load per call.

use std::collections::VecDeque;
use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

/// Environment variable holding the `host:port` of the monitor to send events to.
pub const MONITOR_ENV_VAR: &str = "PACT_FFI_MONITOR";

/// Maximum length of a single formatted argument or result.
const MAX_VALUE_LEN: usize = 2048;
/// Maximum number of events queued towards the background thread.
const MAX_QUEUE: u64 = 10_000;
/// Maximum number of events buffered while no monitor is connected; replayed once one connects.
const MAX_PENDING: usize = 20_000;
/// How often to retry connecting while there is no monitor.
const RECONNECT_INTERVAL: Duration = Duration::from_secs(1);

static SENDER: OnceLock<Option<Mutex<Sender<String>>>> = OnceLock::new();
static SEQ: AtomicU64 = AtomicU64::new(0);
static QUEUED: AtomicU64 = AtomicU64::new(0);
/// Events discarded because the buffers were full; reported to the monitor once connected.
static DROPPED: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u128 {
  SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or_default()
}

fn connect(addr: &str) -> Option<TcpStream> {
  let target = addr.to_socket_addrs().ok()?.next()?;
  let stream = TcpStream::connect_timeout(&target, Duration::from_millis(500)).ok()?;
  let _ = stream.set_nodelay(true);
  Some(stream)
}

fn truncate(mut s: String) -> String {
  if s.len() > MAX_VALUE_LEN {
    let mut end = MAX_VALUE_LEN;
    while !s.is_char_boundary(end) {
      end -= 1;
    }
    s.truncate(end);
    s.push('…');
  }
  s
}

fn sender() -> Option<&'static Mutex<Sender<String>>> {
  SENDER.get_or_init(|| {
    let addr = std::env::var(MONITOR_ENV_VAR).ok().filter(|v| !v.trim().is_empty())?;
    let (tx, rx) = channel::<String>();
    let hello = json!({
      "type": "hello",
      "pid": std::process::id(),
      "version": env!("CARGO_PKG_VERSION"),
      "exe": std::env::current_exe().ok().map(|p| p.to_string_lossy().to_string()),
      "ts_ms": now_ms() as u64
    }).to_string();
    std::thread::Builder::new().name("pact-ffi-monitor".into()).spawn(move || {
      let mut conn: Option<TcpStream> = None;
      let mut last_attempt: Option<Instant> = None;
      // Events wait here until a monitor is connected, so a monitor started late still sees them.
      let mut pending: VecDeque<String> = VecDeque::new();
      loop {
        let received = if pending.is_empty() {
          rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
        } else {
          rx.recv_timeout(RECONNECT_INTERVAL)
        };
        match received {
          Ok(line) => pending.push_back(line),
          Err(RecvTimeoutError::Timeout) => {}
          Err(RecvTimeoutError::Disconnected) => break
        }
        while let Ok(line) = rx.try_recv() {
          pending.push_back(line);
        }
        QUEUED.store(0, Ordering::Relaxed);
        while pending.len() > MAX_PENDING {
          pending.pop_front();
          DROPPED.fetch_add(1, Ordering::Relaxed);
        }

        if conn.is_none() && last_attempt.map(|t| t.elapsed() >= RECONNECT_INTERVAL).unwrap_or(true) {
          last_attempt = Some(Instant::now());
          conn = connect(addr.trim()).and_then(|mut c| writeln!(c, "{}", hello).ok().map(|_| c));
        }
        if let Some(c) = conn.as_mut() {
          let dropped = DROPPED.swap(0, Ordering::Relaxed);
          let mut ok = dropped == 0
            || writeln!(c, "{}", json!({ "type": "dropped", "count": dropped })).is_ok();
          while ok {
            match pending.front() {
              Some(line) => {
                ok = writeln!(c, "{}", line).is_ok();
                if ok {
                  pending.pop_front();
                }
              }
              None => break
            }
          }
          if !ok {
            conn = None;
          }
        }
      }
    }).ok()?;
    Some(Mutex::new(tx))
  }).as_ref()
}

/// Returns true if monitoring is enabled.
pub fn enabled() -> bool {
  sender().is_some()
}

/// Reports an error message set by the FFI (the one `pactffi_get_error_message` would return).
/// The monitor attaches it to the next call finishing on this thread, i.e. the one that failed.
pub fn error(message: &str) {
  if !enabled() {
    return;
  }
  let thread = std::thread::current();
  let event = json!({
    "type": "error",
    "pid": std::process::id(),
    "ts_ms": now_ms() as u64,
    "thread": thread.name().map(|n| n.to_string()).unwrap_or_else(|| format!("{:?}", thread.id())),
    "message": truncate(message.to_string())
  });
  send(event.to_string());
}

fn send(line: String) {
  if QUEUED.load(Ordering::Relaxed) >= MAX_QUEUE {
    DROPPED.fetch_add(1, Ordering::Relaxed);
    return;
  }
  if let Some(tx) = sender() {
    if let Ok(tx) = tx.lock() {
      if tx.send(line).is_ok() {
        QUEUED.fetch_add(1, Ordering::Relaxed);
      }
    }
  }
}

/// An in-flight FFI call. Created by [`start`] and completed with [`Call::finish`].
#[derive(Debug)]
pub struct Call {
  function: &'static str,
  module: &'static str,
  args: Vec<(&'static str, String)>,
  started: Instant,
  started_ms: u128,
}

/// Records the start of an FFI call. Returns `None` (cheaply) when monitoring is disabled.
pub fn start(
  function: &'static str,
  module: &'static str,
  args: impl FnOnce() -> Vec<(&'static str, String)>
) -> Option<Call> {
  if !enabled() {
    return None;
  }
  Some(Call { function, module, args: args(), started: Instant::now(), started_ms: now_ms() })
}

impl Call {
  /// Completes the call, sending the event to the monitor.
  pub fn finish(self, result: impl FnOnce() -> String, panicked: bool) {
    let duration_us = self.started.elapsed().as_micros() as u64;
    let thread = std::thread::current();
    let event: Value = json!({
      "type": "call",
      "seq": SEQ.fetch_add(1, Ordering::Relaxed),
      "pid": std::process::id(),
      "ts_ms": self.started_ms as u64,
      "duration_us": duration_us,
      "thread": thread.name().map(|n| n.to_string()).unwrap_or_else(|| format!("{:?}", thread.id())),
      "function": self.function,
      "module": self.module,
      "args": self.args.iter().map(|(n, v)| json!([n, v])).collect::<Vec<_>>(),
      "result": truncate(result()),
      "panicked": panicked
    });
    send(event.to_string());
  }
}

/// Wrapper used with autoref-based specialisation to pick the best formatting for a value.
#[doc(hidden)]
#[derive(Debug)]
pub struct Wrap<'a, T: ?Sized>(pub &'a T);

/// Most specific: C string pointers are shown as the string they point to.
#[doc(hidden)]
pub trait ArgCStr { fn arg_fmt(self) -> String; }

fn cstr_to_string(ptr: *const libc::c_char) -> String {
  if ptr.is_null() {
    "null".to_string()
  } else {
    // The functions we are monitoring already require these to be valid NULL terminated strings.
    let s = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_string_lossy().to_string();
    truncate(format!("{:?}", s))
  }
}

impl ArgCStr for &&Wrap<'_, *const libc::c_char> {
  fn arg_fmt(self) -> String { cstr_to_string(*self.0) }
}

impl ArgCStr for &&Wrap<'_, *mut libc::c_char> {
  fn arg_fmt(self) -> String { cstr_to_string(*self.0 as *const libc::c_char) }
}

/// Anything implementing `Debug`.
#[doc(hidden)]
pub trait ArgDebug { fn arg_fmt(self) -> String; }

impl<T: std::fmt::Debug + ?Sized> ArgDebug for &Wrap<'_, T> {
  fn arg_fmt(self) -> String { truncate(format!("{:?}", self.0)) }
}

/// Fallback for types without `Debug`.
#[doc(hidden)]
pub trait ArgAny { fn arg_fmt(self) -> String; }

impl<T: ?Sized> ArgAny for Wrap<'_, T> {
  fn arg_fmt(self) -> String { format!("<{}>", std::any::type_name::<T>()) }
}

/// Formats a value for the monitor: C strings as text, `Debug` where available, else the type name.
#[doc(hidden)]
#[macro_export]
macro_rules! monitor_arg {
  ($e:expr) => {{
    #[allow(unused_imports)]
    use $crate::monitor::{ArgAny, ArgCStr, ArgDebug};
    (&&$crate::monitor::Wrap(&$e)).arg_fmt()
  }};
}

/// Guard for hand-written FFI functions: reports the call when dropped. Created with `monitor_call!`.
#[derive(Debug)]
pub struct CallGuard {
  call: Option<Call>,
  result: Option<String>,
}

impl CallGuard {
  /// Records the result to be reported with the call.
  pub fn result(&mut self, result: impl FnOnce() -> String) {
    if self.call.is_some() {
      self.result = Some(result());
    }
  }
}

impl Drop for CallGuard {
  fn drop(&mut self) {
    if let Some(call) = self.call.take() {
      let result = self.result.take().unwrap_or_else(|| "(not captured)".to_string());
      call.finish(|| result, std::thread::panicking());
    }
  }
}

/// Starts a guarded call. Inert (no allocation) when monitoring is disabled.
pub fn guard(
  function: &'static str,
  module: &'static str,
  args: impl FnOnce() -> Vec<(&'static str, String)>
) -> CallGuard {
  CallGuard { call: start(function, module, args), result: None }
}

/// Reports a call to a hand-written FFI function. Place at the top of the function body:
/// `let _monitor = monitor_call!(pactffi_name; arg1, arg2);`
#[doc(hidden)]
#[macro_export]
macro_rules! monitor_call {
  ($name:ident; $($arg:ident),*) => {
    $crate::monitor::guard(stringify!($name), module_path!(), || {
      vec![$((stringify!($arg), $crate::monitor_arg!($arg))),*]
    })
  };
}
