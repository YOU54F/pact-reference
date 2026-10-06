//! Event model and in-memory store.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

/// Maximum number of calls retained overall; oldest are dropped first.
pub const MAX_CALLS: usize = 50_000;
/// Maximum number of calls retained per monitored process, so one chatty process cannot evict the rest.
pub const MAX_CALLS_PER_SESSION: usize = 20_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Event {
  Hello(Hello),
  Call(Call),
  /// An error message set by the FFI; attached to the next call finishing on that thread.
  Error { pid: u32, thread: String, message: String, #[serde(default)] ts_ms: u64 },
  /// The process had to discard events because they were produced faster than they could be sent.
  Dropped { count: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
  pub pid: u32,
  pub version: String,
  pub exe: Option<String>,
  pub ts_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
  Ok,
  Failed,
  Panicked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
  #[allow(dead_code)]
  pub seq: u64,
  pub pid: u32,
  pub ts_ms: u64,
  pub duration_us: u64,
  pub thread: String,
  pub function: String,
  pub module: String,
  pub args: Vec<(String, String)>,
  pub result: String,
  pub panicked: bool,
  /// Error message the FFI recorded while handling this call.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
  /// Unique id assigned by the store (pid/seq are only unique per process).
  #[serde(skip)]
  pub id: u64,
  /// Pact handle this call belongs to, derived from its arguments (or mock server port).
  #[serde(skip)]
  pub pact: Option<u32>,
  /// The call reported failure (false / negative / non-zero status code, or set an error message).
  #[serde(skip)]
  pub failed: bool,
}

impl Call {
  /// When the call returned.
  pub fn end_ms(&self) -> u64 {
    self.ts_ms + self.duration_us / 1000
  }

  /// Start in microseconds since the epoch.
  pub fn ts_us(&self) -> u64 {
    self.ts_ms * 1000
  }

  /// Precise end in microseconds since the epoch, used for sorting.
  pub fn end_us(&self) -> u64 {
    self.ts_ms * 1000 + self.duration_us
  }

  pub fn status(&self) -> Status {
    if self.panicked { Status::Panicked } else if self.failed { Status::Failed } else { Status::Ok }
  }
}

/// Whether a call's result indicates failure, going by the conventions of the FFI return values.
fn is_failure(function: &str, result: &str) -> bool {
  let result = result.trim();
  if result == "false" {
    return true;
  }
  let Ok(n) = result.parse::<i64>() else { return false };
  let status_code_fn = function.contains("write_pact_file") || function.ends_with("write_file")
    || function.contains("verifier_execute") || function == "pactffi_verify"
    || function.contains("free_pact_handle");
  n < 0 || (status_code_fn && n != 0)
}

/// Extracts `name: N` from a Debug-formatted struct such as `PactHandle { pact_ref: 3 }`.
fn debug_field(value: &str, field: &str) -> Option<u64> {
  let rest = &value[value.find(&format!("{}:", field))? + field.len() + 1..];
  let digits: String = rest.trim_start().chars().take_while(|c| c.is_ascii_digit()).collect();
  digits.parse().ok()
}

/// Pact handle from Debug-formatted handle text. Interaction/message handles encode the pact
/// reference in their upper 16 bits.
fn pact_from_text(value: &str) -> Option<u32> {
  debug_field(value, "pact_ref").or_else(|| debug_field(value, "interaction_ref").map(|r| r >> 16)).map(|r| r as u32)
}

fn pact_from_args(args: &[(String, String)]) -> Option<u32> {
  args.iter().find_map(|(_, v)| pact_from_text(v))
}

fn arg<'a>(c: &'a Call, name: &str) -> Option<&'a str> {
  c.args.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
}

/// Removes the quotes from a Debug-formatted string argument.
fn unquote(v: &str) -> Option<String> {
  let v = v.trim();
  v.strip_prefix('"').and_then(|v| v.strip_suffix('"')).map(|v| v.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn port_arg(c: &Call) -> Option<i64> {
  c.args.iter().filter(|(n, _)| n.contains("port")).find_map(|(_, v)| v.trim().parse::<i64>().ok())
}

/// A monitored process: start is when it first connected, end when the connection dropped.
#[derive(Debug, Clone)]
pub struct Session {
  pub hello: Hello,
  pub ended_ms: Option<u64>,
  pub calls: u64,
  /// Calls currently held in the store for this process.
  retained: usize,
}

/// A pact built through the FFI, tracked from `pactffi_new_pact`.
#[derive(Debug, Clone)]
pub struct PactInfo {
  pub pid: u32,
  pub handle: u32,
  pub consumer: Option<String>,
  pub provider: Option<String>,
  pub spec: Option<String>,
  pub interactions: u32,
  pub written: bool,
  pub created_ms: u64,
  pub calls: u32,
  pub failures: u32,
}

impl PactInfo {
  /// "consumer → provider", or just the handle if the names were not seen.
  pub fn title(&self) -> String {
    match (&self.consumer, &self.provider) {
      (Some(c), Some(p)) => format!("{} → {}", c, p),
      _ => format!("pact #{}", self.handle),
    }
  }
}

/// A mock server started for a pact.
#[derive(Debug, Clone)]
pub struct MockServer {
  pub pid: u32,
  pub port: i64,
  pub pact: Option<u32>,
  pub created_ms: u64,
  pub shutdown_ms: Option<u64>,
  /// Result of `pactffi_mock_server_matched`, if it was called.
  pub matched: Option<bool>,
  pub written: bool,
  pub calls: u32,
}

#[derive(Debug, Clone, Default)]
pub struct FunctionStats {
  pub count: u64,
  pub total_us: u64,
  pub max_us: u64,
  pub panics: u64,
  pub failures: u64,
}

impl FunctionStats {
  pub fn avg_us(&self) -> u64 {
    if self.count == 0 { 0 } else { self.total_us / self.count }
  }
}

#[derive(Debug, Default)]
pub struct Store {
  pub sessions: BTreeMap<u32, Session>,
  pub pacts: BTreeMap<(u32, u32), PactInfo>,
  pub mock_servers: Vec<MockServer>,
  pub calls: Vec<Call>,
  pub stats: BTreeMap<String, FunctionStats>,
  pub connections: usize,
  pub total_calls: u64,
  /// Events the monitored processes reported discarding.
  pub dropped: u64,
  /// Errors waiting for the call that caused them, by (pid, thread).
  pending_errors: HashMap<(u32, String), String>,
  next_id: u64,
}

impl Store {
  pub fn apply(&mut self, event: Event) {
    match event {
      Event::Hello(h) => {
        // Reconnects re-send hello; keep the original start time and counters.
        self.sessions.entry(h.pid).and_modify(|s| s.ended_ms = None)
          .or_insert(Session { hello: h, ended_ms: None, calls: 0, retained: 0 });
      }
      Event::Error { pid, thread, message, .. } => {
        self.pending_errors.insert((pid, thread), message);
      }
      Event::Dropped { count } => self.dropped += count,
      Event::Call(c) => self.apply_call(c),
    }
  }

  fn latest_mock_server(&mut self, pid: u32, port: i64) -> Option<&mut MockServer> {
    self.mock_servers.iter_mut().rev().find(|m| m.pid == pid && m.port == port)
  }

  fn apply_call(&mut self, mut c: Call) {
    if c.error.is_none() {
      c.error = self.pending_errors.remove(&(c.pid, c.thread.clone()));
    }
    c.failed = is_failure(&c.function, &c.result) || c.error.is_some();
    c.pact = pact_from_args(&c.args).or_else(|| pact_from_text(&c.result));

    let creates_mock_server = c.function.contains("create_mock_server");
    let port = port_arg(&c);
    if !creates_mock_server && c.pact.is_none() {
      c.pact = port.and_then(|p| self.latest_mock_server(c.pid, p).and_then(|m| m.pact));
    }

    if c.function == "pactffi_new_pact" && !c.failed {
      if let Some(handle) = c.pact {
        self.pacts.insert((c.pid, handle), PactInfo {
          pid: c.pid, handle,
          consumer: arg(&c, "consumer_name").and_then(unquote),
          provider: arg(&c, "provider_name").and_then(unquote),
          spec: None, interactions: 0, written: false, created_ms: c.ts_ms, calls: 0, failures: 0,
        });
      }
    }
    if let Some(info) = c.pact.and_then(|h| self.pacts.get_mut(&(c.pid, h))) {
      info.calls += 1;
      info.failures += c.failed as u32;
      if c.function.contains("new_") && c.function.contains("interaction") && !c.failed {
        info.interactions += 1;
      }
      if c.function == "pactffi_with_specification" && !c.failed {
        info.spec = arg(&c, "version").map(|v| v.to_string());
      }
      if c.function == "pactffi_pact_handle_write_file" && !c.failed {
        info.written = true;
      }
    }

    if creates_mock_server {
      if let Ok(port) = c.result.trim().parse::<i64>() {
        if port > 0 {
          self.mock_servers.push(MockServer {
            pid: c.pid, port, pact: c.pact, created_ms: c.ts_ms, shutdown_ms: None, matched: None,
            written: false, calls: 0,
          });
        }
      }
    } else if let Some(port) = port {
      let (function, result, end_ms, failed) = (c.function.clone(), c.result.trim().to_string(), c.end_ms(), c.failed);
      let mut written_pact = None;
      if let Some(m) = self.latest_mock_server(c.pid, port) {
        m.calls += 1;
        if function.contains("cleanup_mock_server") || function.contains("shutdown_mock_server") {
          m.shutdown_ms = Some(end_ms);
        } else if function.contains("mock_server_matched") {
          m.matched = Some(result == "true");
        } else if function.contains("write_pact_file") && !failed {
          m.written = true;
          written_pact = m.pact;
        }
      }
      if let Some(info) = written_pact.and_then(|h| self.pacts.get_mut(&(c.pid, h))) {
        info.written = true;
      }
    }

    let pid = c.pid;
    if let Some(sess) = self.sessions.get_mut(&pid) {
      sess.calls += 1;
      sess.retained += 1;
    }
    c.id = self.next_id;
    self.next_id += 1;
    self.total_calls += 1;
    let s = self.stats.entry(c.function.clone()).or_default();
    s.count += 1;
    s.total_us += c.duration_us;
    s.max_us = s.max_us.max(c.duration_us);
    s.panics += c.panicked as u64;
    s.failures += c.failed as u64;
    self.calls.push(c);

    if self.calls.len() > MAX_CALLS {
      self.drop_oldest(None, MAX_CALLS / 10);
    }
    let retained = self.sessions.get(&pid).map(|s| s.retained).unwrap_or(0);
    if retained > MAX_CALLS_PER_SESSION + MAX_CALLS_PER_SESSION / 10 {
      self.drop_oldest(Some(pid), MAX_CALLS_PER_SESSION / 10);
    }
  }

  /// Removes the `n` oldest calls, optionally only those of one process.
  fn drop_oldest(&mut self, pid: Option<u32>, n: usize) {
    let mut left = n;
    let mut removed: HashMap<u32, usize> = HashMap::new();
    self.calls.retain(|c| {
      if left > 0 && pid.map_or(true, |p| p == c.pid) {
        left -= 1;
        *removed.entry(c.pid).or_default() += 1;
        false
      } else {
        true
      }
    });
    for (pid, n) in removed {
      if let Some(s) = self.sessions.get_mut(&pid) {
        s.retained = s.retained.saturating_sub(n);
      }
    }
  }

  pub fn end_session(&mut self, pid: u32, now_ms: u64) {
    if let Some(sess) = self.sessions.get_mut(&pid) {
      sess.ended_ms = Some(now_ms);
    }
  }

  pub fn clear(&mut self) {
    self.calls.clear();
    self.stats.clear();
    self.pacts.clear();
    self.mock_servers.clear();
    self.pending_errors.clear();
    self.total_calls = 0;
    self.dropped = 0;
    self.sessions.values_mut().for_each(|s| { s.calls = 0; s.retained = 0 });
  }

  /// Events to write out for an export: sessions first, then every retained call.
  pub fn export(&self) -> Vec<Event> {
    self.sessions.values().map(|s| Event::Hello(s.hello.clone()))
      .chain(self.calls.iter().cloned().map(Event::Call)).collect()
  }

  /// Indices of calls matching `filter` (see [`Filter`]); `only_problems` keeps panics and failures.
  pub fn filtered(&self, filter: &str, only_problems: bool) -> Vec<usize> {
    let filter = Filter::parse(filter);
    self.calls.iter().enumerate()
      .filter(|(_, c)| (!only_problems || c.status() != Status::Ok) && filter.matches(c))
      .map(|(i, _)| i).collect()
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
  Any,
  Function,
  Pact,
  Thread,
  Pid,
  Arg,
  Result,
  Status,
  Error,
}

#[derive(Debug, Clone)]
struct Term {
  field: Field,
  value: String,
  negate: bool,
}

/// A search expression. Whitespace separated terms must all match. A term is free text (matched
/// against the function, arguments, result and error) or `field:value` with the fields `fn`,
/// `pact`, `thread`, `pid`, `arg`, `result`, `status` (ok, fail, panic) and `error`. Prefix a term
/// with `-` to negate it. For example: `fn:with_body pact:3 -thread:main status:fail`.
#[derive(Debug, Clone, Default)]
pub struct Filter {
  terms: Vec<Term>,
}

impl Filter {
  pub fn parse(text: &str) -> Filter {
    let terms = text.split_whitespace().filter_map(|token| {
      let (negate, token) = match token.strip_prefix('-') {
        Some(rest) if !rest.is_empty() => (true, rest),
        _ => (false, token),
      };
      let (field, value) = match token.split_once(':') {
        Some((f, v)) => {
          let field = match f.to_lowercase().as_str() {
            "fn" | "function" => Some(Field::Function),
            "pact" => Some(Field::Pact),
            "thread" => Some(Field::Thread),
            "pid" => Some(Field::Pid),
            "arg" => Some(Field::Arg),
            "result" => Some(Field::Result),
            "status" => Some(Field::Status),
            "error" => Some(Field::Error),
            _ => None,
          };
          match field {
            Some(field) => (field, v),
            None => (Field::Any, token),
          }
        }
        None => (Field::Any, token),
      };
      if value.is_empty() { None } else { Some(Term { field, value: value.to_lowercase(), negate }) }
    }).collect();
    Filter { terms }
  }

  pub fn matches(&self, c: &Call) -> bool {
    self.terms.iter().all(|t| t.matches(c) != t.negate)
  }
}

impl Term {
  fn matches(&self, c: &Call) -> bool {
    let v = &self.value;
    match self.field {
      Field::Function => c.function.to_lowercase().contains(v),
      Field::Pact => c.pact.map(|p| p.to_string() == v.trim_start_matches('#')).unwrap_or(false),
      Field::Thread => c.thread.to_lowercase().contains(v),
      Field::Pid => c.pid.to_string() == *v,
      Field::Arg => c.args.iter().any(|(n, a)| n.to_lowercase().contains(v) || a.to_lowercase().contains(v)),
      Field::Result => c.result.to_lowercase().contains(v),
      Field::Error => c.error.as_ref().map(|e| e.to_lowercase().contains(v)).unwrap_or(false),
      Field::Status => match v.as_str() {
        "ok" => c.status() == Status::Ok,
        "fail" | "failed" | "failure" => c.status() == Status::Failed,
        "panic" | "panicked" => c.status() == Status::Panicked,
        "problem" | "problems" => c.status() != Status::Ok,
        _ => false,
      },
      Field::Any => {
        c.function.to_lowercase().contains(v)
          || c.result.to_lowercase().contains(v)
          || c.error.as_ref().map(|e| e.to_lowercase().contains(v)).unwrap_or(false)
          || c.args.iter().any(|(n, a)| n.to_lowercase().contains(v) || a.to_lowercase().contains(v))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn call_with(function: &str, args: serde_json::Value, result: &str, thread: &str) -> Event {
    serde_json::from_value(serde_json::json!({
      "type": "call", "seq": 0, "pid": 1, "ts_ms": 100, "duration_us": 1, "thread": thread,
      "function": function, "module": "m", "args": args, "result": result, "panicked": false
    })).unwrap()
  }

  fn with(function: &str, args: serde_json::Value, result: &str) -> Event {
    call_with(function, args, result, "t")
  }

  fn call(function: &str, us: u64, panicked: bool) -> Event {
    serde_json::from_value(serde_json::json!({
      "type": "call", "seq": 0, "pid": 1, "ts_ms": 0, "duration_us": us, "thread": "t",
      "function": function, "module": "m", "args": [["a", "1"]], "result": "ok", "panicked": panicked
    })).unwrap()
  }

  #[test]
  fn aggregates_and_filters() {
    let mut s = Store::default();
    s.apply(call("pactffi_new_pact", 10, false));
    s.apply(call("pactffi_new_pact", 30, true));
    s.apply(call("pactffi_with_header", 5, false));
    let st = &s.stats["pactffi_new_pact"];
    assert_eq!((st.count, st.avg_us(), st.max_us, st.panics), (2, 20, 30, 1));
    assert_eq!(s.filtered("header", false), vec![2]);
    assert_eq!(s.filtered("", true), vec![1]);
  }

  #[test]
  fn derives_pact_handle() {
    let mut s = Store::default();
    s.apply(with("pactffi_new_interaction", serde_json::json!([["pact", "PactHandle { pact_ref: 3 }"]]), "x"));
    s.apply(with("pactffi_with_header", serde_json::json!([["interaction", "InteractionHandle { interaction_ref: 196609 }"]]), "true"));
    s.apply(with("pactffi_create_mock_server_for_pact", serde_json::json!([["pact", "PactHandle { pact_ref: 3 }"]]), "1234"));
    s.apply(with("pactffi_mock_server_matched", serde_json::json!([["mock_server_port", "1234"]]), "true"));
    s.apply(with("pactffi_version", serde_json::json!([]), "\"1\""));
    let pacts: Vec<_> = s.calls.iter().map(|c| c.pact).collect();
    assert_eq!(pacts, vec![Some(3), Some(3), Some(3), Some(3), None]);
  }

  #[test]
  fn tracks_pacts_and_mock_servers() {
    let mut s = Store::default();
    s.apply(with("pactffi_new_pact", serde_json::json!([["consumer_name", "\"web\""], ["provider_name", "\"api\""]]), "PactHandle { pact_ref: 7 }"));
    s.apply(with("pactffi_with_specification", serde_json::json!([["pact", "PactHandle { pact_ref: 7 }"], ["version", "V4"]]), "true"));
    s.apply(with("pactffi_new_interaction", serde_json::json!([["pact", "PactHandle { pact_ref: 7 }"]]), "InteractionHandle { interaction_ref: 458753 }"));
    s.apply(with("pactffi_create_mock_server_for_transport", serde_json::json!([["pact", "PactHandle { pact_ref: 7 }"]]), "4000"));
    s.apply(with("pactffi_mock_server_matched", serde_json::json!([["mock_server_port", "4000"]]), "false"));
    s.apply(with("pactffi_write_pact_file", serde_json::json!([["mock_server_port", "4000"]]), "0"));
    s.apply(with("pactffi_cleanup_mock_server", serde_json::json!([["mock_server_port", "4000"]]), "true"));

    // new_pact itself is attributed to the pact it created.
    assert_eq!(s.calls[0].pact, Some(7));
    let p = &s.pacts[&(1, 7)];
    assert_eq!(p.title(), "web → api");
    assert_eq!((p.interactions, p.written, p.spec.as_deref()), (1, true, Some("V4")));
    let m = &s.mock_servers[0];
    assert_eq!((m.port, m.pact, m.matched, m.written), (4000, Some(7), Some(false), true));
    assert!(m.shutdown_ms.is_some());
    // matched() returning false is a failure.
    assert_eq!(s.calls[4].status(), Status::Failed);
    assert_eq!(s.calls[5].status(), Status::Ok);
  }

  #[test]
  fn attaches_errors_to_the_failing_call_on_the_same_thread() {
    let mut s = Store::default();
    s.apply(Event::Error { pid: 1, thread: "a".into(), message: "boom".into(), ts_ms: 0 });
    s.apply(call_with("pactffi_with_body", serde_json::json!([]), "true", "b"));
    s.apply(call_with("pactffi_with_body", serde_json::json!([]), "true", "a"));
    s.apply(call_with("pactffi_with_body", serde_json::json!([]), "true", "a"));
    assert_eq!(s.calls[0].error, None);
    assert_eq!(s.calls[1].error.as_deref(), Some("boom"));
    assert_eq!(s.calls[1].status(), Status::Failed);
    assert_eq!(s.calls[2].error, None);
  }

  #[test]
  fn search_syntax() {
    let mut s = Store::default();
    s.apply(with("pactffi_with_body", serde_json::json!([["pact", "PactHandle { pact_ref: 3 }"], ["body", "\"hello\""]]), "true"));
    s.apply(call_with("pactffi_with_body", serde_json::json!([["pact", "PactHandle { pact_ref: 4 }"]]), "false", "main"));
    s.apply(with("pactffi_new_pact", serde_json::json!([]), "x"));
    assert_eq!(s.filtered("fn:with_body", false), vec![0, 1]);
    assert_eq!(s.filtered("pact:3", false), vec![0]);
    assert_eq!(s.filtered("fn:with_body -pact:3", false), vec![1]);
    assert_eq!(s.filtered("thread:main", false), vec![1]);
    assert_eq!(s.filtered("status:fail", false), vec![1]);
    assert_eq!(s.filtered("hello", false), vec![0]);
    // Unknown prefixes are plain text.
    assert!(s.filtered("nope:1", false).is_empty());
  }

  #[test]
  fn caps_calls_per_session() {
    let mut s = Store::default();
    s.apply(Event::Hello(Hello { pid: 1, version: "x".into(), exe: None, ts_ms: 0 }));
    for _ in 0..(MAX_CALLS_PER_SESSION + MAX_CALLS_PER_SESSION / 10 + 1) {
      s.apply(call("f", 1, false));
    }
    assert!(s.calls.len() <= MAX_CALLS_PER_SESSION + 1);
    assert_eq!(s.sessions[&1].retained, s.calls.len());
  }

  #[test]
  fn export_round_trips() {
    let mut s = Store::default();
    s.apply(Event::Hello(Hello { pid: 1, version: "x".into(), exe: None, ts_ms: 5 }));
    s.apply(Event::Error { pid: 1, thread: "t".into(), message: "bad".into(), ts_ms: 0 });
    s.apply(with("pactffi_with_body", serde_json::json!([["a", "1"]]), "true"));
    let text: Vec<String> = s.export().iter().map(|e| serde_json::to_string(e).unwrap()).collect();
    let mut t = Store::default();
    for line in &text {
      t.apply(serde_json::from_str(line).unwrap());
    }
    assert_eq!(t.calls.len(), 1);
    assert_eq!(t.calls[0].error.as_deref(), Some("bad"));
    assert_eq!(t.sessions[&1].hello.ts_ms, 5);
  }
}
