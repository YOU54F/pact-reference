//! Interface to a standard HTTP mock server provided by Pact

use std::{env, thread};
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use itertools::Itertools;

use pact_models::pact::Pact;
#[cfg(feature = "plugins")] use pact_models::plugins::PluginData;
#[cfg(feature = "plugins")] use pact_plugin_driver::plugin_manager::{drop_plugin_access, increment_plugin_access};
#[cfg(feature = "plugins")] use pact_plugin_driver::plugin_models::{PluginDependency, PluginDependencyType};
use tracing::{debug, warn};
use url::Url;
use uuid::Uuid;

use pact_matching::metrics::{MetricEvent, send_metrics};
use pact_mock_server::matching::MatchResult;
use pact_mock_server::mock_server;
use pact_mock_server::mock_server::{MockServerConfig, MockServerMetrics};
use pact_models::v4::http_parts::HttpRequest;

use crate::mock_server::ValidatingMockServer;
use crate::util::panic_or_print_error;

/// A mock HTTP server that handles the requests described in a `Pact`, intended
/// for use in tests, and validates that the requests made to that server are
/// correct. This wraps the standard Pact HTTP mock server.
///
/// Because this is intended for use in tests, it will panic if something goes
/// wrong.
pub struct ValidatingHttpMockServer {
  // A description of our mock server, for use in error messages.
  description: String,
  // The URL of our mock server.
  url: Url,
  // The mock server instance
  mock_server: Arc<Mutex<mock_server::MockServer>>,
  // Signal received when the server thread is done executing
  done_rx: std::sync::mpsc::Receiver<()>,
  // Output directory to write pact files
  output_dir: Option<PathBuf>,
  // overwrite or merge Pact files
  overwrite: bool
}

impl ValidatingHttpMockServer {
  /// Create a new mock server which handles requests as described in the
  /// pact, and runs in a background thread
  ///
  /// Panics:
  /// Will panic if the provided Pact can not be sent to the background thread.
  pub fn start(pact: Box<dyn Pact + Send + Sync>, output_dir: Option<PathBuf>) -> Box<dyn ValidatingMockServer> {
    debug!("Starting mock server from pact {:?}", pact);

    #[allow(unused_variables)] let plugin_data = pact.plugin_data();
    #[cfg(feature = "plugins")] Self::increment_plugin_access(&plugin_data);

    // Spawn new runtime in thread to prevent reactor execution context conflict
    let (pact_tx, pact_rx) = std::sync::mpsc::channel::<Box<dyn Pact + Send + Sync>>();
    pact_tx.send(pact).expect("INTERNAL ERROR: Could not pass pact into mock server thread");
    let (mock_server, done_rx) = std::thread::spawn(|| {
      let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("new runtime");

      let (mock_server, server_future) = runtime.block_on(async move {
        mock_server::MockServer::new(
          Uuid::new_v4().to_string(),
          pact_rx.recv().unwrap(),
          ([0, 0, 0, 0], 0).into(),
          MockServerConfig::default()
        )
          .await
          .unwrap()
      });

      // Start the actual thread the runtime will run on
      let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
      let tname = format!(
        "test({})-pact-mock-server",
        thread::current().name().unwrap_or("<unknown>")
      );
      thread::Builder::new()
        .name(tname)
        .spawn(move || {
          runtime.block_on(server_future);
          let _ = done_tx.send(());
          #[cfg(feature = "plugins")] Self::decrement_plugin_access(&plugin_data);
        })
        .expect("thread spawn");

      (mock_server, done_rx)
    })
      .join()
      .unwrap();

    let (description, url_str) = {
      let ms = mock_server.lock().unwrap();
      let pact = ms.pact.as_ref();
      let description = format!(
        "{}/{}", pact.consumer().name, pact.provider().name
      );
      (description, ms.url())
    };
    Box::new(ValidatingHttpMockServer {
      description,
      url: url_str.parse().expect("invalid mock server URL"),
      mock_server,
      done_rx,
      output_dir,
      overwrite: false
    })
  }

  #[cfg(feature = "plugins")]
  fn decrement_plugin_access(plugins: &Vec<PluginData>) {
    for plugin in plugins {
      let dependency = PluginDependency {
        name: plugin.name.clone(),
        version: Some(plugin.version.clone()),
        dependency_type: PluginDependencyType::Plugin
      };
      drop_plugin_access(&dependency);
    }
  }

  #[cfg(feature = "plugins")]
  fn increment_plugin_access(plugins: &Vec<PluginData>) {
    for plugin in plugins {
      let dependency = PluginDependency {
        name: plugin.name.clone(),
        version: Some(plugin.version.clone()),
        dependency_type: PluginDependencyType::Plugin
      };
      increment_plugin_access(&dependency);
    }
  }

  /// Create a new mock server which handles requests as described in the
  /// pact, and runs in a background task in the current Tokio runtime.
  ///
  /// Panics:
  /// Will panic if unable to get the URL to the spawned mock server
  pub async fn start_async(pact: Box<dyn Pact + Send + Sync>, output_dir: Option<PathBuf>) -> Box<dyn ValidatingMockServer> {
    debug!("Starting mock server from pact {:?}", pact);

    #[allow(unused_variables)] let plugin_data = pact.plugin_data();
    #[cfg(feature = "plugins")] Self::increment_plugin_access(&plugin_data);

    let (mock_server, server_future) = mock_server::MockServer::new(
      Uuid::new_v4().to_string(),
      pact,
      ([0, 0, 0, 0], 0 as u16).into(),
      MockServerConfig::default()
    )
      .await
      .unwrap();

    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    tokio::spawn(async move {
      server_future.await;
      let _ = done_tx.send(());
      #[cfg(feature = "plugins")] Self::decrement_plugin_access(&plugin_data);
    });

    let (description, url_str) = {
      let ms = mock_server.lock().unwrap();
      let pact = ms.pact.as_ref();
      let description = format!(
        "{}/{}", pact.consumer().name, pact.provider().name
      );
      (description, ms.url())
    };
    Box::new(ValidatingHttpMockServer {
      description,
      url: url_str.parse().expect("invalid mock server URL"),
      mock_server,
      done_rx,
      output_dir,
      overwrite: false
    })
  }

  /// Helper function called by our `drop` implementation. This basically exists
  /// so that it can return `Err(message)` whenever needed without making the
  /// flow control in `drop` ultra-complex.
  fn drop_helper(&mut self) -> Result<(), String> {
    // Kill the server
    let mut ms = self.mock_server.lock().unwrap();
    ms.shutdown()?;

    // Wait for the server thread to finish
    if let Err(_) = self.done_rx.recv_timeout(std::time::Duration::from_secs(3)) {
      warn!("Timed out waiting for mock server to finish");
    }

    // Send any metrics in another thread as this thread could be panicking due to an assertion.
    let interactions = {
      let pact = ms.pact.as_ref();
      pact.interactions().len()
    };
    thread::spawn(move || {
      send_metrics(MetricEvent::ConsumerTestRun {
        interactions,
        test_framework: "pact_consumer".to_string(),
        app_name: "pact_consumer".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string()
      });
    });

    // Look up any mismatches which occurred.
    let mismatches = ms.mismatches();

    if mismatches.is_empty() {
      // Success! Write out the generated pact file.
      let output_dir = self.output_dir.as_ref()
        .map(|dir| {
          let dir = dir.to_string_lossy().to_string();
          if dir.is_empty() { None } else { Some(dir) }
        })
        .flatten()
        .unwrap_or_else(|| {
          let val = env::var("PACT_OUTPUT_DIR");
          debug!("env:PACT_OUTPUT_DIR = {:?}", val);
          val.unwrap_or_else(|_| "target/pacts".to_owned())
        });
      debug!("Pact output_dir = '{}'", output_dir);
      let overwrite = env::var("PACT_OVERWRITE")
        .map(|v| {
          debug!("env:PACT_OVERWRITE = {:?}", v);
          v == "true"
        })
        .ok()
        .unwrap_or(self.overwrite);
      ms.write_pact(&Some(output_dir), overwrite)
        .map_err(|err| format!("error writing pact: {}", err))?;
      Ok(())
    } else {
      // Failure. Format our errors.
      let size = termsize::get().map(|sz| sz.cols).unwrap_or(120) - 2;
      let pad = "-".repeat(size as usize);
      let mut msg = format!(" {} \nMock server {} failed verification:\n", pad, self.description);
      for mismatch in mismatches {
        match mismatch {
          MatchResult::RequestMatch(..) => {
            warn!("list of mismatches contains a match");
          }
          MatchResult::RequestMismatch(request, _, mismatches) => {
            let _ = writeln!(&mut msg, "\n  - request {}:\n", request);
            for m in mismatches {
              let _ = writeln!(&mut msg, "    - {}", m.description());
            }
          }
          MatchResult::RequestNotFound(request) => {
            let _ = writeln!(&mut msg, "\n  - received unexpected request {}:\n", short_description(&request));
            let debug_str = format!("{:#?}", request);
            let _ = writeln!(&mut msg, "{}", debug_str.lines().map(|ln| format!("      {}", ln)).join("\n"));
          }
          MatchResult::MissingRequest(request) => {
            let _ = writeln!(
              &mut msg,
              "\n  - request {} expected, but never occurred:\n", short_description(&request),
            );
            let debug_str = format!("{:#?}", request);
            let _ = writeln!(&mut msg, "{}", debug_str.lines().map(|ln| format!("      {}", ln)).join("\n"));
          }
        }
      }
      let _ = writeln!(&mut msg, " {} ", pad);
      Err(msg)
    }
  }
}

// TODO: Implement this in the HTTP request struct
fn short_description(request: &HttpRequest) -> String {
  format!("{} {}", request.method.to_uppercase(), request.path)
}

impl ValidatingMockServer for ValidatingHttpMockServer {
  fn url(&self) -> Url {
    self.url.clone()
  }

  fn path(&self, path: &str) -> Url {
    // We panic here because this a _test_ library, the `?` operator is
    // useless in tests, and filling up our test code with piles of `unwrap`
    // calls is ugly.
    self.url.join(path.as_ref()).expect("could not parse URL")
  }

  fn status(&self) -> Vec<MatchResult> {
    self.mock_server.lock().unwrap().mismatches()
  }

  fn metrics(&self) -> MockServerMetrics {
    self.mock_server.lock().unwrap().metrics.clone()
  }
}

impl Drop for ValidatingHttpMockServer {
  fn drop(&mut self) {
    let result = self.drop_helper();
    if let Err(msg) = result {
      panic_or_print_error(&msg);
    }
  }
}
