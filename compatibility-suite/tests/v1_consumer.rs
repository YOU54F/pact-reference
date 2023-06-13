use cucumber::World;
use tracing_subscriber::EnvFilter;

use crate::v1_steps::consumer::ConsumerWorld;

pub mod v1_steps {
  pub mod common;
  pub mod consumer;
}

#[tokio::main]
async fn main() {
  let format = tracing_subscriber::fmt::format().pretty();
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .event_format(format)
    .init();

  ConsumerWorld::cucumber()
    .fail_on_skipped()
    .before(|_feature, _, scenario, world| Box::pin(async move {
      world.scenario_id = scenario.name.clone();
    }))
    .filter_run_and_exit("pact-compatibility-suite/features/V1", |feature, _rule, _scenario| {
      feature.tags.iter().any(|tag| tag == "consumer")
    })
    .await;
}
