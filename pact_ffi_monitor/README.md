# pact_ffi_monitor

A [Freya](https://github.com/marc2332/freya) GUI that shows, live, every call made to the `pact_ffi`
shared library: function, arguments, result, duration, thread and whether it panicked.

## How it works

`pact_ffi` has an opt-in monitor (`pact_ffi/src/monitor`). If the `PACT_FFI_MONITOR` environment
variable is set to `host:port`, each call to an FFI function declared with the `ffi_fn!` macro is sent
as a line of JSON over TCP to that address. When unset, the cost is one atomic load per call.
Any language binding (JVM, Node, Python, Go, ...) works since it's the library itself reporting.

## Usage

```sh
cargo run --release                 # listens on 127.0.0.1:7777 (or pass an address as the first arg)

# in the process under test (must load a pact_ffi built from this repo)
PACT_FFI_MONITOR=127.0.0.1:7777 <your test command>
```

Open a saved capture with `cargo run --release -- --import FILE`.

Try it without another app: `cd rust/pact_ffi && PACT_FFI_MONITOR=127.0.0.1:7777 cargo run --example monitor_demo`
(multi-threaded consumer tests, one deliberately failing).

## UI

- **Calls**: start and finish time of each call (click for arguments, result, status and error message).
  *Group by* thread, pact handle, or both (either order); groups collapse and show call counts, time
  span and failures. Click any column header to sort (click again to reverse). A call is attributed to a
  pact handle from the `PactHandle`/`InteractionHandle` it receives or returns, so the several mock
  servers a pact spawns group together.
- **Timeline**: one lane per thread, a bar per call on a time axis, zoomable; click a bar for details.
  Shows the newest 3000 filtered calls.
- **Pacts**: pacts (consumer/provider, spec, interactions, written) and mock servers (port, pact,
  created/shut down, matched, written).
- **Functions**: count, avg/max duration, failures, panics. **Sessions**: one row per process.
- **Search** (toolbar): free text, or `fn:` `pact:` `thread:` `pid:` `arg:` `result:` `error:`
  `status:fail|panic|ok`; prefix `-` to exclude. *Problems* shows only failed/panicked calls.
  A call is *failed* when it set an FFI error message or returned a failure value (null, negative, false).
- **Export / Import** NDJSON via the file box; **Times** toggles UTC/local; Pause freezes the view.
- Footer reports dropped events (the app produced calls faster than they could be sent).

## Coverage

- Functions declared with `ffi_fn!` report arguments and results.
- Hand-written `extern "C"` functions report arguments and duration via `monitor_call!`
  (result shown as "(not captured)"). New FFI functions should use one of the two.
- `*const c_char` / `*mut c_char` arguments and results are shown as the string they point to.

## Limitations

- Other pointer arguments (e.g. byte buffers) are shown as addresses.
- Values are truncated at 2 KiB; the monitor keeps the latest 50,000 calls, at most 20,000 per process.
- Events are buffered in the app (up to 20,000) until a monitor connects, then replayed; the app
  reconnects every second if the monitor restarts. Beyond the buffer, events are dropped and counted.
- Failure detection and pact/mock-server tracking are heuristics based on arguments and results.
- Built against Freya `0.5.0-rc.8` (pre-release).
