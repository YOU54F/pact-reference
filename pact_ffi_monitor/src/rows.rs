//! Turns the flat list of calls into the sorted, grouped rows shown in the Calls view.

use std::cmp::Ordering;
use std::collections::HashSet;

use std::collections::BTreeMap;

use crate::model::{Call, PactInfo, Status};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortKey {
  Start,
  End,
  Function,
  Thread,
  Pact,
  Duration,
  Status,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
  Thread,
  Pact,
}

/// Preset group-by choices offered in the toolbar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GroupBy {
  None,
  Thread,
  Pact,
  ThreadThenPact,
  PactThenThread,
}

impl GroupBy {
  pub const ALL: [GroupBy; 5] = [GroupBy::None, GroupBy::Thread, GroupBy::Pact, GroupBy::ThreadThenPact, GroupBy::PactThenThread];

  pub fn label(&self) -> &'static str {
    match self {
      GroupBy::None => "None",
      GroupBy::Thread => "Thread",
      GroupBy::Pact => "Pact",
      GroupBy::ThreadThenPact => "Thread › Pact",
      GroupBy::PactThenThread => "Pact › Thread",
    }
  }

  pub fn levels(&self) -> &'static [Level] {
    match self {
      GroupBy::None => &[],
      GroupBy::Thread => &[Level::Thread],
      GroupBy::Pact => &[Level::Pact],
      GroupBy::ThreadThenPact => &[Level::Thread, Level::Pact],
      GroupBy::PactThenThread => &[Level::Pact, Level::Thread],
    }
  }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Row {
  Group {
    /// Unique path, used for collapsing and as a stable key.
    path: String,
    label: String,
    depth: usize,
    count: usize,
    panics: usize,
    failures: usize,
    start_ms: u64,
    end_ms: u64,
    collapsed: bool,
  },
  /// Index into the store's call list.
  Call { index: usize, depth: usize },
}

/// Stable key (for collapsing) and display label of the group `call` belongs to at `level`.
fn level_key(call: &Call, level: Level, pacts: &BTreeMap<(u32, u32), PactInfo>) -> (String, String) {
  match level {
    Level::Thread => (format!("thread:{}:{}", call.pid, call.thread), format!("{} · pid {}", call.thread, call.pid)),
    Level::Pact => match call.pact {
      Some(p) => {
        let label = match pacts.get(&(call.pid, p)) {
          Some(info) => format!(
            "{} · #{} · {} interaction{}{}{} · pid {}",
            info.title(), p, info.interactions, if info.interactions == 1 { "" } else { "s" },
            info.spec.as_ref().map(|s| format!(" · {}", s)).unwrap_or_default(),
            if info.written { " · written" } else { "" },
            call.pid,
          ),
          None => format!("Pact #{} · pid {}", p, call.pid),
        };
        (format!("pact:{}:{}", call.pid, p), label)
      }
      None => (format!("pact:{}:none", call.pid), format!("(no pact) · pid {}", call.pid)),
    },
  }
}

pub fn compare(a: &Call, b: &Call, key: SortKey) -> Ordering {
  let ord = match key {
    SortKey::Start => (a.ts_ms, a.pid, a.seq).cmp(&(b.ts_ms, b.pid, b.seq)),
    SortKey::End => a.end_us().cmp(&b.end_us()),
    SortKey::Function => a.function.cmp(&b.function),
    SortKey::Thread => a.thread.cmp(&b.thread),
    SortKey::Pact => a.pact.cmp(&b.pact),
    SortKey::Duration => a.duration_us.cmp(&b.duration_us),
    SortKey::Status => a.status().cmp(&b.status()),
  };
  // Tie-break on arrival order so sorting is stable and deterministic.
  ord.then(a.id.cmp(&b.id))
}

/// Builds the rows for `indices` (into `calls`), grouped by `levels` and sorted by `key`.
pub fn build(
  calls: &[Call],
  indices: &[usize],
  levels: &[Level],
  key: SortKey,
  descending: bool,
  collapsed: &HashSet<String>,
  pacts: &BTreeMap<(u32, u32), PactInfo>,
) -> Vec<Row> {
  let mut sorted = indices.to_vec();
  sorted.sort_by(|&a, &b| {
    let o = compare(&calls[a], &calls[b], key);
    if descending { o.reverse() } else { o }
  });
  let mut rows = Vec::with_capacity(sorted.len() + 16);
  emit(calls, sorted, levels, 0, "", collapsed, pacts, &mut rows);
  rows
}

#[allow(clippy::too_many_arguments)]
fn emit(
  calls: &[Call],
  sorted: Vec<usize>,
  levels: &[Level],
  depth: usize,
  parent: &str,
  collapsed: &HashSet<String>,
  pacts: &BTreeMap<(u32, u32), PactInfo>,
  rows: &mut Vec<Row>,
) {
  let Some((&level, rest)) = levels.split_first() else {
    rows.extend(sorted.into_iter().map(|index| Row::Call { index, depth }));
    return;
  };

  // Bucket preserving the sort order; groups are ordered by their first call under the active sort.
  let mut order: Vec<(String, String)> = Vec::new();
  let mut buckets: std::collections::HashMap<String, Vec<usize>> = Default::default();
  for i in sorted {
    let (k, label) = level_key(&calls[i], level, pacts);
    buckets.entry(k.clone()).or_insert_with(|| { order.push((k.clone(), label)); Vec::new() }).push(i);
  }
  for (k, label) in order {
    let members = buckets.remove(&k).unwrap();
    let path = format!("{}/{}", parent, k);
    let is_collapsed = collapsed.contains(&path);
    rows.push(Row::Group {
      path: path.clone(),
      label,
      depth,
      count: members.len(),
      panics: members.iter().filter(|&&i| calls[i].status() == Status::Panicked).count(),
      failures: members.iter().filter(|&&i| calls[i].status() == Status::Failed).count(),
      start_ms: members.iter().map(|&i| calls[i].ts_ms).min().unwrap_or(0),
      end_ms: members.iter().map(|&i| calls[i].end_ms()).max().unwrap_or(0),
      collapsed: is_collapsed,
    });
    if !is_collapsed {
      emit(calls, members, rest, depth + 1, &path, collapsed, pacts, rows);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn call(id: u64, ts: u64, thread: &str, pact: Option<u32>, dur: u64) -> Call {
    Call {
      seq: id, pid: 1, ts_ms: ts, duration_us: dur, thread: thread.into(), function: format!("f{}", id),
      module: String::new(), args: vec![], result: String::new(), panicked: false, error: None, failed: false, id, pact,
    }
  }

  fn sample() -> Vec<Call> {
    vec![call(0, 10, "a", Some(1), 5), call(1, 20, "b", Some(2), 50), call(2, 30, "a", Some(2), 1), call(3, 40, "b", Some(1), 9)]
  }

  #[test]
  fn sorts_ungrouped() {
    let c = sample();
    let rows = build(&c, &[0, 1, 2, 3], &[], SortKey::Duration, true, &Default::default(), &Default::default());
    let order: Vec<_> = rows.iter().map(|r| match r { Row::Call { index, .. } => *index, _ => 99 }).collect();
    assert_eq!(order, vec![1, 3, 0, 2]);
  }

  #[test]
  fn groups_nested_thread_then_pact() {
    let c = sample();
    let rows = build(&c, &[0, 1, 2, 3], &[Level::Thread, Level::Pact], SortKey::Start, false, &Default::default(), &Default::default());
    // a: pact1{0}, pact2{2}; b: pact2{1}, pact1{3}
    let shape: Vec<String> = rows.iter().map(|r| match r {
      Row::Group { depth, count, .. } => format!("G{}:{}", depth, count),
      Row::Call { index, .. } => format!("c{}", index),
    }).collect();
    assert_eq!(shape, ["G0:2", "G1:1", "c0", "G1:1", "c2", "G0:2", "G1:1", "c1", "G1:1", "c3"]);
  }

  #[test]
  fn collapsed_groups_hide_children() {
    let c = sample();
    let mut collapsed = HashSet::new();
    collapsed.insert("/thread:1:a".to_string());
    let rows = build(&c, &[0, 1, 2, 3], &[Level::Thread], SortKey::Start, false, &collapsed, &Default::default());
    assert_eq!(rows.len(), 4); // 2 headers + the two calls in b
    assert!(matches!(rows[0], Row::Group { collapsed: true, .. }));
  }
}
