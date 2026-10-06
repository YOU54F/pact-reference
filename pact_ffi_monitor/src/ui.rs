use std::collections::{BTreeMap, HashSet};

use freya::prelude::*;
use freya::sdk::use_track_watcher;
use tokio::sync::watch;

use crate::model::Status;
use crate::persist;
use crate::rows::{self, GroupBy, Row, SortKey};
use crate::server::Shared;
use crate::time::fmt_time;

const ROW_HEIGHT: f32 = 26.0;
const BG: (u8, u8, u8) = (24, 26, 31);
const PANEL: (u8, u8, u8) = (33, 36, 43);
const TEXT: (u8, u8, u8) = (225, 228, 235);
const MUTED: (u8, u8, u8) = (140, 146, 160);
const ACCENT: (u8, u8, u8) = (96, 165, 250);
const BAD: (u8, u8, u8) = (248, 113, 113);
const WARN: (u8, u8, u8) = (251, 191, 36);
const GOOD: (u8, u8, u8) = (74, 222, 128);
const SELECTED: (u8, u8, u8) = (49, 66, 99);
const SEARCH_HELP: &str = "Search: text, fn:  pact:  thread:  pid:  arg:  result:  error:  status:fail|panic|ok  (prefix - to exclude)";
const DEFAULT_EXPORT_PATH: &str = "pact_ffi_monitor.ndjson";
/// Most calls drawn on the timeline at once (the newest), to keep it responsive.
const TIMELINE_MAX_CALLS: usize = 3000;
const TIMELINE_BASE_WIDTH: f32 = 900.0;
const LANE_HEIGHT: f32 = 22.0;

pub struct MonitorApp {
  pub store: Shared,
  pub rx: watch::Receiver<u64>,
  pub addr: String,
  pub listen_error: Option<String>,
  /// Used to refresh the display after changes made from the UI (e.g. an import).
  pub notify: watch::Sender<u64>,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
  Calls,
  Functions,
  Sessions,
  Timeline,
  Pacts,
}

fn fmt_duration(us: u64) -> String {
  if us >= 1_000_000 { format!("{:.2} s", us as f64 / 1e6) }
  else if us >= 1_000 { format!("{:.2} ms", us as f64 / 1e3) }
  else { format!("{} µs", us) }
}

fn cell(text: impl Into<String>, width: Size, color: (u8, u8, u8)) -> Rect {
  rect().width(width).child(label().text(text.into()).color(color).font_size(13.).max_lines(1))
}

impl App for MonitorApp {
  fn render(&self) -> impl IntoElement {
    use_track_watcher(&self.rx);

    let filter = use_state(String::new);
    let mut selected = use_state(|| None::<u64>);
    let mut tab = use_state(|| Tab::Calls);
    let mut paused = use_state(|| false);
    let mut only_problems = use_state(|| false);
    // While paused we freeze what is displayed by remembering the call count at pause time.
    let mut frozen_len = use_state(|| None::<usize>);
    let mut group_by = use_state(|| GroupBy::None);
    let sort_key = use_state(|| SortKey::Start);
    let descending = use_state(|| true);
    let collapsed = use_state(HashSet::<String>::new);
    let zoom = use_state(|| 1.0f32);
    let export_path = use_state(|| DEFAULT_EXPORT_PATH.to_string());
    let mut message = use_state(String::new);
    // Changes when the time zone is toggled, so everything showing times re-renders.
    let mut time_toggles = use_state(|| 0u32);

    let store = self.store.clone();
    let version = *self.rx.borrow();
    let filter_text = filter.read().clone();
    let problems = *only_problems.read();
    let toggles = *time_toggles.read();

    let (indices, rows, total, connections, sessions, panics, failures, dropped) = {
      let s = store.lock().unwrap();
      let mut idx = s.filtered(&filter_text, problems);
      if let Some(n) = *frozen_len.read() {
        idx.retain(|i| *i < n);
      }
      let panics: u64 = s.stats.values().map(|f| f.panics).sum();
      let failures: u64 = s.stats.values().map(|f| f.failures).sum();
      let rows = rows::build(&s.calls, &idx, group_by.read().levels(), *sort_key.read(), *descending.read(), &collapsed.read(), &s.pacts);
      (idx, rows, s.total_calls, s.connections, s.sessions.len(), panics, failures, s.dropped)
    };

    let status = match &self.listen_error {
      Some(e) => (format!("Cannot listen on {}: {}", self.addr, e), BAD),
      None if connections > 0 => (format!("{} process(es) connected · listening on {}", connections, self.addr), GOOD),
      None => (format!("Waiting for connections on {} — run your app with PACT_FFI_MONITOR={}", self.addr, self.addr), MUTED),
    };

    let current_tab = *tab.read();
    let tab_button = move |t: Tab, name: &'static str| {
      let b = Button::new().on_press(move |_| { tab.set(t); }).child(name);
      if t == current_tab { b.filled() } else { b.outline() }
    };

    let toolbar = rect().horizontal().width(Size::fill()).cross_align(Alignment::center()).spacing(8.).padding(10.)
      .background(PANEL)
      .child(label().text("Pact FFI Monitor").font_size(16.).font_weight(FontWeight::BOLD).color(TEXT))
      .child(tab_button(Tab::Calls, "Calls"))
      .child(tab_button(Tab::Timeline, "Timeline"))
      .child(tab_button(Tab::Pacts, "Pacts"))
      .child(tab_button(Tab::Functions, "Functions"))
      .child(tab_button(Tab::Sessions, "Sessions"))
      .child(Input::new(filter).placeholder(SEARCH_HELP).width(Size::px(300.)))
      .child({
        let store = store.clone();
        Button::new().on_press(move |_| {
          let now = !*paused.read();
          paused.set(now);
          frozen_len.set(if now { Some(store.lock().unwrap().calls.len()) } else { None });
        }).child(if *paused.read() { "Resume" } else { "Pause" })
      })
      .child(
        Button::new().on_press(move |_| { let v = *only_problems.read();
          only_problems.set(!v); })
          .child(if problems { "Problems ✓" } else { "Problems" })
      )
      .child({
        let store = store.clone();
        Button::new().on_press(move |_| {
          store.lock().unwrap().clear();
          selected.set(None);
          frozen_len.set(if *paused.read() { Some(0) } else { None });
        }).child("Clear")
      });

    let current_group = *group_by.read();
    let group_bar = GroupBy::ALL.iter().fold(
      rect().horizontal().width(Size::fill()).cross_align(Alignment::center()).spacing(6.).padding((4., 10.)).background(PANEL)
        .child(label().text("Group by").color(MUTED).font_size(12.)),
      |bar, &g| {
        let b = Button::new().on_press(move |_| { group_by.set(g); }).child(g.label());
        bar.child(if g == current_group { b.filled() } else { b.outline() })
      },
    );

    let body = match current_tab {
      Tab::Calls => calls_view(store.clone(), rows, (version, toggles), selected, sort_key, descending, collapsed,
        *sort_key.read(), *descending.read(), group_bar),
      Tab::Timeline => timeline_view(store.clone(), &indices, selected, zoom),
      Tab::Pacts => pacts_view(store.clone()).into(),
      Tab::Sessions => sessions_view(store.clone()).into(),
      Tab::Functions => functions_view(store.clone(), filter_text.clone()).into(),
    };

    let actions = rect().horizontal().width(Size::fill()).cross_align(Alignment::center()).spacing(8.).padding((4., 10.)).background(PANEL)
      .child(label().text("File").color(MUTED).font_size(12.))
      .child(Input::new(export_path).width(Size::px(260.)))
      .child({
        let store = store.clone();
        Button::new().on_press(move |_| {
          let path = export_path.read().clone();
          message.set(match persist::export(&path, &store.lock().unwrap()) {
            Ok(n) => format!("Exported {} events to {}", n, path),
            Err(e) => format!("Export to {} failed: {}", path, e),
          });
        }).child("Export")
      })
      .child({
        let store = store.clone();
        let notify = self.notify.clone();
        Button::new().on_press(move |_| {
          let path = export_path.read().clone();
          message.set(match persist::import(&path, &mut store.lock().unwrap()) {
            Ok(n) => format!("Imported {} events from {}", n, path),
            Err(e) => format!("Import from {} failed: {}", path, e),
          });
          notify.send_modify(|v| *v += 1);
        }).child("Import")
      })
      .child(
        Button::new().on_press(move |_| { crate::time::toggle_utc(); time_toggles.set(toggles + 1); })
          .child(if crate::time::is_utc() { "Times: UTC" } else { "Times: local" })
      )
      .child(label().text(message.read().clone()).color(MUTED).font_size(12.));

    let mut footer = rect().horizontal().width(Size::fill()).spacing(16.).padding((6., 10.)).background(PANEL)
      .child(label().text(status.0).color(status.1).font_size(12.))
      .child(label().text(format!(
        "{} calls · {} shown · {} process(es) seen · {} failed · {} panics",
        total, indices.len(), sessions, failures, panics
      )).color(MUTED).font_size(12.));
    if dropped > 0 {
      footer = footer.child(label().text(format!("{} events dropped by the app (too many too fast)", dropped)).color(BAD).font_size(12.));
    }

    rect().expanded().background(BG).color(TEXT).vertical()
      .child(toolbar)
      .child(actions)
      .child(rect().height(Size::flex(1.)).width(Size::fill()).child(body))
      .child(footer)
  }
}

fn sort_header(
  title: &'static str, key: SortKey, width: Size,
  mut sort_key: State<SortKey>, mut descending: State<bool>, cur: SortKey, desc: bool,
) -> Rect {
  let active = cur == key;
  let arrow = if !active { "" } else if desc { " ▼" } else { " ▲" };
  rect().width(width)
    .on_press(move |_| {
      if *sort_key.read() == key {
        let d = *descending.read();
        descending.set(!d);
      } else {
        sort_key.set(key);
        // Times and durations are most useful newest/largest first; text ascending.
        descending.set(matches!(key, SortKey::Start | SortKey::End | SortKey::Duration));
      }
    })
    .child(label().text(format!("{}{}", title, arrow)).color(if active { ACCENT } else { MUTED }).font_size(13.).max_lines(1))
}

#[allow(clippy::too_many_arguments)]
fn calls_view(
  store: Shared, rows: Vec<Row>, version: (u64, u32), mut selected: State<Option<u64>>,
  sort_key: State<SortKey>, descending: State<bool>, mut collapsed: State<HashSet<String>>,
  cur: SortKey, desc: bool, group_bar: Rect,
) -> Element {
  let sel = *selected.read();
  let len = rows.len();
  let collapsed_n = collapsed.read().len();

  let header = rect().horizontal().padding((4., 10.)).background(PANEL)
    .child(sort_header("Start", SortKey::Start, Size::px(90.), sort_key, descending, cur, desc))
    .child(sort_header("End", SortKey::End, Size::px(90.), sort_key, descending, cur, desc))
    .child(sort_header("Function", SortKey::Function, Size::px(300.), sort_key, descending, cur, desc))
    .child(sort_header("Thread", SortKey::Thread, Size::px(110.), sort_key, descending, cur, desc))
    .child(sort_header("Pact", SortKey::Pact, Size::px(60.), sort_key, descending, cur, desc))
    .child(sort_header("Duration", SortKey::Duration, Size::px(90.), sort_key, descending, cur, desc))
    .child(sort_header("Status", SortKey::Status, Size::px(60.), sort_key, descending, cur, desc));

  let list_store = store.clone();
  let list_rows = rows.clone();
  let list = VirtualScrollView::new_with_data((sel, version, len, cur, desc, collapsed_n), move |item, _| {
    let Some(row) = list_rows.get(item.index) else { return rect().key(item.index).into() };
    match row {
      Row::Group { path, label: name, depth, count, panics, failures, start_ms, end_ms, collapsed: is_collapsed } => {
        let path = path.clone();
        let text = format!(
          "{} {}  ·  {} call{}  ·  {} → {}  ·  {}{}",
          if *is_collapsed { "▶" } else { "▼" }, name, count, if *count == 1 { "" } else { "s" },
          fmt_time(*start_ms), fmt_time(*end_ms), fmt_duration((end_ms - start_ms) * 1000),
          if *panics > 0 { format!("  ·  {} panic(s)", panics) } else { String::new() },
        );
        let text = if *failures > 0 { format!("{}  ·  {} failed", text, failures) } else { text };
        rect().key(format!("g{}", path)).horizontal().width(Size::fill()).height(Size::px(ROW_HEIGHT))
          .padding((4., 10. + *depth as f32 * 18.)).cross_align(Alignment::center())
          .background(PANEL)
          .on_press(move |_| {
            let mut set = collapsed.read().clone();
            if !set.remove(&path) { set.insert(path.clone()); }
            collapsed.set(set);
          })
          .child(label().text(text).color(if *panics > 0 { BAD } else if *failures > 0 { WARN } else { ACCENT }).font_size(13.).max_lines(1))
          .into()
      }
      Row::Call { index, depth } => {
        let s = list_store.lock().unwrap();
        let Some(c) = s.calls.get(*index) else { return rect().key(item.index).into() };
        let id = c.id;
        let color = match c.status() { Status::Panicked => BAD, Status::Failed => WARN, Status::Ok => TEXT };
        rect().key(id).horizontal().width(Size::fill()).height(Size::px(ROW_HEIGHT))
          .padding((4., 10. + *depth as f32 * 18.)).cross_align(Alignment::center())
          .background(if Some(id) == sel { SELECTED } else { BG })
          .on_press(move |_| { selected.set(Some(id)); })
          .child(cell(fmt_time(c.ts_ms), Size::px(90.), MUTED))
          .child(cell(fmt_time(c.end_ms()), Size::px(90.), MUTED))
          .child(cell(c.function.clone(), Size::px(300.), color))
          .child(cell(c.thread.clone(), Size::px(110.), MUTED))
          .child(cell(c.pact.map(|p| format!("#{}", p)).unwrap_or_else(|| "—".into()), Size::px(60.), MUTED))
          .child(cell(fmt_duration(c.duration_us), Size::px(90.), MUTED))
          .child(status_cell(c.status()))
          .into()
      }
    }
  })
  .length(len)
  .item_size(ROW_HEIGHT);

  let detail = detail_view(store, sel);

  rect().expanded().horizontal()
    .child(rect().width(Size::percent(66.)).height(Size::fill()).vertical().child(group_bar).child(header).child(list))
    .child(rect().width(Size::flex(1.)).height(Size::fill()).background(PANEL).padding(12.).child(detail))
    .into()
}

fn kv(name: &str, value: impl Into<String>, color: (u8, u8, u8)) -> Rect {
  rect().vertical().spacing(2.)
    .child(label().text(name.to_string()).color(MUTED).font_size(11.))
    .child(paragraph().span(Span::new(value.into()).color(color).font_size(13.)))
}

fn detail_view(store: Shared, sel: Option<u64>) -> Element {
  let s = store.lock().unwrap();
  let Some(c) = sel.and_then(|id| s.calls.iter().rev().find(|c| c.id == id)) else {
    return label().text("Select a call to see its details").color(MUTED).into();
  };
  let mut col = rect().vertical().spacing(10.)
    .child(label().text(c.function.clone()).font_size(16.).font_weight(FontWeight::BOLD).color(ACCENT))
    .child(kv("Module", c.module.clone(), TEXT))
    .child(kv("Started", fmt_time(c.ts_ms), TEXT))
    .child(kv("Finished", fmt_time(c.end_ms()), TEXT))
    .child(kv("Duration", fmt_duration(c.duration_us), TEXT))
    .child(kv("Process / thread", format!("pid {} · {}", c.pid, c.thread), TEXT))
    .child(kv("Status", match c.status() { Status::Ok => "ok", Status::Failed => "failed", Status::Panicked => "panicked" },
      match c.status() { Status::Ok => GOOD, Status::Failed => WARN, Status::Panicked => BAD }))
    .child(kv("Pact", c.pact.and_then(|h| s.pacts.get(&(c.pid, h))).map(|p| format!("#{} · {}", p.handle, p.title()))
      .or_else(|| c.pact.map(|p| format!("#{}", p))).unwrap_or_else(|| "—".into()), TEXT));
  for (n, v) in &c.args {
    col = col.child(kv(&format!("arg {}", n), v.clone(), TEXT));
  }
  col = col.child(kv(if c.panicked { "Result (PANICKED, fallback value)" } else { "Result" }, c.result.clone(),
    if c.panicked { BAD } else { TEXT }));
  if let Some(e) = &c.error {
    col = col.child(kv("Error message", e.clone(), BAD));
  }
  ScrollView::new().child(col).into()
}

fn sessions_view(store: Shared) -> Rect {
  let s = store.lock().unwrap();
  let header = rect().horizontal().padding((4., 10.)).background(PANEL)
    .child(cell("PID", Size::px(70.), MUTED))
    .child(cell("Executable", Size::flex(1.), MUTED))
    .child(cell("Version", Size::px(80.), MUTED))
    .child(cell("Started", Size::px(100.), MUTED))
    .child(cell("Finished", Size::px(100.), MUTED))
    .child(cell("Elapsed", Size::px(90.), MUTED))
    .child(cell("Calls", Size::px(70.), MUTED));
  let list = s.sessions.values().rev().fold(rect().vertical(), |acc, se| {
    let finished = se.ended_ms.map(fmt_time).unwrap_or_else(|| "running".into());
    let elapsed = se.ended_ms.map(|e| fmt_duration(e.saturating_sub(se.hello.ts_ms) * 1000)).unwrap_or_else(|| "—".into());
    acc.child(
      rect().key(se.hello.pid).horizontal().padding((4., 10.)).height(Size::px(ROW_HEIGHT))
        .child(cell(se.hello.pid.to_string(), Size::px(70.), TEXT))
        .child(cell(se.hello.exe.clone().unwrap_or_default(), Size::flex(1.), TEXT))
        .child(cell(se.hello.version.clone(), Size::px(80.), MUTED))
        .child(cell(fmt_time(se.hello.ts_ms), Size::px(100.), MUTED))
        .child(cell(finished, Size::px(100.), if se.ended_ms.is_none() { (74, 222, 128) } else { MUTED }))
        .child(cell(elapsed, Size::px(90.), MUTED))
        .child(cell(se.calls.to_string(), Size::px(70.), TEXT)),
    )
  });
  rect().expanded().vertical().child(header).child(ScrollView::new().child(list))
}

fn functions_view(store: Shared, filter: String) -> Rect {
  let s = store.lock().unwrap();
  let f = filter.trim().to_lowercase();
  let mut rows: Vec<_> = s.stats.iter().filter(|(n, _)| f.is_empty() || n.to_lowercase().contains(&f)).collect();
  rows.sort_by(|a, b| b.1.count.cmp(&a.1.count));

  let header = rect().horizontal().padding((4., 10.)).background(PANEL)
    .child(cell("Function", Size::flex(1.), MUTED))
    .child(cell("Calls", Size::px(80.), MUTED))
    .child(cell("Avg", Size::px(100.), MUTED))
    .child(cell("Max", Size::px(100.), MUTED))
    .child(cell("Failures", Size::px(80.), MUTED))
    .child(cell("Panics", Size::px(80.), MUTED));

  let list = rows.into_iter().fold(rect().vertical(), |acc, (name, st)| {
    acc.child(
      rect().key(name.clone()).horizontal().padding((4., 10.)).height(Size::px(ROW_HEIGHT))
        .child(cell(name.clone(), Size::flex(1.), TEXT))
        .child(cell(st.count.to_string(), Size::px(80.), TEXT))
        .child(cell(fmt_duration(st.avg_us()), Size::px(100.), TEXT))
        .child(cell(fmt_duration(st.max_us), Size::px(100.), TEXT))
        .child(cell(st.failures.to_string(), Size::px(80.), if st.failures > 0 { WARN } else { MUTED }))
        .child(cell(st.panics.to_string(), Size::px(80.), if st.panics > 0 { BAD } else { MUTED })),
    )
  });
  rect().expanded().vertical().child(header).child(ScrollView::new().child(list))
}

fn status_cell(status: Status) -> Rect {
  let (text, color) = match status {
    Status::Ok => ("ok", TEXT),
    Status::Failed => ("FAIL", WARN),
    Status::Panicked => ("PANIC", BAD),
  };
  cell(text, Size::px(60.), color)
}

fn pacts_view(store: Shared) -> Rect {
  let s = store.lock().unwrap();
  let section = |title: &str| rect().padding((8., 10.)).child(label().text(title.to_string()).color(ACCENT).font_size(14.).font_weight(FontWeight::BOLD));

  let pact_header = rect().horizontal().padding((4., 10.)).background(PANEL)
    .child(cell("Handle", Size::px(70.), MUTED))
    .child(cell("Pact", Size::flex(1.), MUTED))
    .child(cell("Spec", Size::px(70.), MUTED))
    .child(cell("Interactions", Size::px(100.), MUTED))
    .child(cell("Written", Size::px(80.), MUTED))
    .child(cell("Calls", Size::px(70.), MUTED))
    .child(cell("Failed", Size::px(70.), MUTED))
    .child(cell("Created", Size::px(100.), MUTED));
  let pacts = s.pacts.values().rev().fold(rect().vertical(), |acc, p| {
    acc.child(rect().key(format!("{}-{}", p.pid, p.handle)).horizontal().padding((4., 10.)).height(Size::px(ROW_HEIGHT))
      .child(cell(format!("#{}", p.handle), Size::px(70.), TEXT))
      .child(cell(format!("{} (pid {})", p.title(), p.pid), Size::flex(1.), TEXT))
      .child(cell(p.spec.clone().unwrap_or_else(|| "—".into()), Size::px(70.), MUTED))
      .child(cell(p.interactions.to_string(), Size::px(100.), TEXT))
      .child(cell(if p.written { "yes" } else { "no" }, Size::px(80.), if p.written { GOOD } else { MUTED }))
      .child(cell(p.calls.to_string(), Size::px(70.), TEXT))
      .child(cell(p.failures.to_string(), Size::px(70.), if p.failures > 0 { WARN } else { MUTED }))
      .child(cell(fmt_time(p.created_ms), Size::px(100.), MUTED)))
  });

  let ms_header = rect().horizontal().padding((4., 10.)).background(PANEL)
    .child(cell("Port", Size::px(80.), MUTED))
    .child(cell("Pact", Size::flex(1.), MUTED))
    .child(cell("Created", Size::px(100.), MUTED))
    .child(cell("State", Size::px(100.), MUTED))
    .child(cell("Matched", Size::px(90.), MUTED))
    .child(cell("Written", Size::px(80.), MUTED))
    .child(cell("Calls", Size::px(70.), MUTED));
  let servers = s.mock_servers.iter().enumerate().rev().fold(rect().vertical(), |acc, (i, m)| {
    let pact = m.pact.and_then(|h| s.pacts.get(&(m.pid, h))).map(|p| format!("#{} · {}", p.handle, p.title()))
      .or_else(|| m.pact.map(|h| format!("#{}", h))).unwrap_or_else(|| "—".into());
    let (matched, mc) = match m.matched { Some(true) => ("yes", GOOD), Some(false) => ("NO", BAD), None => ("—", MUTED) };
    acc.child(rect().key(i).horizontal().padding((4., 10.)).height(Size::px(ROW_HEIGHT))
      .child(cell(m.port.to_string(), Size::px(80.), TEXT))
      .child(cell(pact, Size::flex(1.), TEXT))
      .child(cell(fmt_time(m.created_ms), Size::px(100.), MUTED))
      .child(cell(m.shutdown_ms.map(|t| format!("shut {}", fmt_time(t))).unwrap_or_else(|| "running".into()), Size::px(100.), if m.shutdown_ms.is_some() { MUTED } else { GOOD }))
      .child(cell(matched, Size::px(90.), mc))
      .child(cell(if m.written { "yes" } else { "no" }, Size::px(80.), if m.written { GOOD } else { MUTED }))
      .child(cell(m.calls.to_string(), Size::px(70.), TEXT)))
  });

  rect().expanded().vertical()
    .child(ScrollView::new().child(
      rect().vertical()
        .child(section("Pacts"))
        .child(pact_header).child(pacts)
        .child(section("Mock servers"))
        .child(ms_header).child(servers)
    ))
}

/// Calls drawn as bars on a time axis, one lane per (process, thread).
fn timeline_view(store: Shared, indices: &[usize], mut selected: State<Option<u64>>, mut zoom: State<f32>) -> Element {
  let sel = *selected.read();
  let z = *zoom.read();
  let s = store.lock().unwrap();
  let start = indices.len().saturating_sub(TIMELINE_MAX_CALLS);
  let shown: Vec<_> = indices[start..].iter().filter_map(|i| s.calls.get(*i)).collect();

  let zoom_bar = rect().horizontal().width(Size::fill()).cross_align(Alignment::center()).spacing(6.).padding((4., 10.)).background(PANEL)
    .child(label().text("Zoom").color(MUTED).font_size(12.))
    .child(Button::new().on_press(move |_| { let v = *zoom.read(); zoom.set((v / 2.).max(0.25)); }).child("−"))
    .child(label().text(format!("{}×", z)).color(TEXT).font_size(12.))
    .child(Button::new().on_press(move |_| { let v = *zoom.read(); zoom.set((v * 2.).min(256.)); }).child("+"))
    .child(label().text(format!(
      "{} calls shown{}", shown.len(),
      if start > 0 { format!(" (newest {} of {})", TIMELINE_MAX_CALLS, indices.len()) } else { String::new() }
    )).color(MUTED).font_size(12.));

  let body: Element = if shown.is_empty() {
    rect().padding(12.).child(label().text("No calls to show").color(MUTED)).into()
  } else {
    let t0 = shown.iter().map(|c| c.ts_us()).min().unwrap_or(0);
    let t1 = shown.iter().map(|c| c.end_us()).max().unwrap_or(t0 + 1).max(t0 + 1);
    let width = TIMELINE_BASE_WIDTH * z;
    let scale = width / (t1 - t0) as f32;

    // Lanes ordered by their first call.
    let mut lanes: BTreeMap<(u32, String), (u64, Vec<&crate::model::Call>)> = BTreeMap::new();
    for c in &shown {
      let e = lanes.entry((c.pid, c.thread.clone())).or_insert((c.ts_us(), Vec::new()));
      e.0 = e.0.min(c.ts_us());
      e.1.push(c);
    }
    let mut lanes: Vec<_> = lanes.into_iter().collect();
    lanes.sort_by_key(|(_, (first, _))| *first);

    let label_w = 150.0;
    let mut axis = rect().width(Size::px(width)).height(Size::px(18.));
    for k in 0..=8 {
      let frac = k as f32 / 8.0;
      let ms = (t0 as f64 + (t1 - t0) as f64 * frac as f64) / 1000.0;
      axis = axis.child(rect().position(Position::new_absolute().left((width * frac).min(width - 90.)).top(0.))
        .child(label().text(fmt_time(ms as u64)).color(MUTED).font_size(10.)));
    }
    let mut col = rect().vertical().spacing(2.)
      .child(rect().horizontal().child(rect().width(Size::px(label_w))).child(axis));
    for ((pid, thread), (_, calls)) in lanes {
      let mut lane = rect().width(Size::px(width)).height(Size::px(LANE_HEIGHT)).background(PANEL);
      for c in calls {
        let id = c.id;
        let color = match c.status() { Status::Panicked => BAD, Status::Failed => WARN, Status::Ok => ACCENT };
        let left = (c.ts_us() - t0) as f32 * scale;
        let w = (c.duration_us as f32 * scale).max(2.0);
        lane = lane.child(
          rect().key(id).position(Position::new_absolute().left(left).top(2.))
            .width(Size::px(w)).height(Size::px(LANE_HEIGHT - 4.))
            .background(if Some(id) == sel { (255, 255, 255) } else { color })
            .on_press(move |_| { selected.set(Some(id)); })
        );
      }
      col = col.child(rect().key(format!("{}-{}", pid, thread)).horizontal()
        .child(rect().width(Size::px(label_w)).child(label().text(format!("{} ({})", thread, pid)).color(MUTED).font_size(12.).max_lines(1)))
        .child(lane));
    }
    ScrollView::new().direction(Direction::Horizontal).child(ScrollView::new().child(col.padding(8.))).into()
  };

  drop(s);
  let detail = detail_view(store.clone(), sel);
  rect().expanded().horizontal()
    .child(rect().width(Size::percent(66.)).height(Size::fill()).vertical().child(zoom_bar).child(body))
    .child(rect().width(Size::flex(1.)).height(Size::fill()).background(PANEL).padding(12.).child(detail))
    .into()
}
