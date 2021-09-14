//! Builder for constructing Asynchronous message interactions

use std::collections::{HashMap, VecDeque};
use std::env;
use std::path::PathBuf;

use bytes::Bytes;
use log::{debug, error, info};
use maplit::hashmap;
use pact_models::content_types::ContentType;
use pact_models::json_utils::json_to_string;
use pact_models::pact::{ReadWritePact, write_pact};
use pact_models::prelude::{MatchingRules, OptionalBody, Pact, ProviderState};
use pact_models::v4::async_message::AsynchronousMessage;
use pact_models::v4::interaction::InteractionMarkup;
use pact_models::v4::message_parts::MessageContents;
use pact_models::v4::pact::V4Pact;
use pact_models::v4::V4InteractionType;
use pact_plugin_driver::catalogue_manager::find_content_matcher;
use pact_plugin_driver::content::{ContentMatcher, InteractionContents};
use pact_plugin_driver::plugin_models::PactPluginManifest;
use serde_json::{json, Value};
use pact_models::plugins::PluginData;

#[derive(Clone, Debug)]
/// Asynchronous message interaction builder. Normally created via PactBuilder::message_interaction.
pub struct MessageInteractionBuilder {
  description: String,
  provider_states: Vec<ProviderState>,
  comments: Vec<String>,
  test_name: Option<String>,
  interaction_type: String,
  message_contents: InteractionContents,
  contents_plugin: Option<PactPluginManifest>
}

impl MessageInteractionBuilder {
  /// Create a new message interaction builder
  pub fn new<D: Into<String>>(description: D, interaction_type: D) -> MessageInteractionBuilder {
    MessageInteractionBuilder {
      description: description.into(),
      provider_states: vec![],
      comments: vec![],
      test_name: None,
      interaction_type: interaction_type.into(),
      message_contents: Default::default(),
      contents_plugin: None
    }
  }

  /// Specify a "provider state" for this interaction. This is normally use to
  /// set up database fixtures when using a pact to test a provider.
  pub fn given<G: Into<String>>(&mut self, given: G) -> &mut Self {
    self.provider_states.push(ProviderState::default(&given.into()));
    self
  }

  /// Adds a text comment to this interaction. This allows to specify just a bit more information
  /// about the interaction. It has no functional impact, but can be displayed in the broker HTML
  /// page, and potentially in the test output.
  pub fn comment<G: Into<String>>(&mut self, comment: G) -> &mut Self {
    self.comments.push(comment.into());
    self
  }

  /// Sets the test name for this interaction. This allows to specify just a bit more information
  /// about the interaction. It has no functional impact, but can be displayed in the broker HTML
  /// page, and potentially in the test output.
  pub fn test_name<G: Into<String>>(&mut self, name: G) -> &mut Self {
    self.test_name = Some(name.into());
    self
  }

  /// The interaction we've built (in V4 format).
  pub fn build(&self) -> AsynchronousMessage {
    debug!("Building V4 AsynchronousMessage interaction: {:?}", self);

    let mut rules = MatchingRules::default();
    rules.add_category("body")
      .add_rules(self.message_contents.rules.as_ref().cloned().unwrap_or_default());
    AsynchronousMessage {
      id: None,
      key: None,
      description: self.description.clone(),
      provider_states: self.provider_states.clone(),
      contents: MessageContents {
        contents: self.message_contents.body.clone(),
        metadata: self.message_contents.metadata.as_ref().cloned().unwrap_or_default(),
        matching_rules: rules,
        generators: self.message_contents.generators.as_ref().cloned().unwrap_or_default()
      },
      comments: hashmap!{
        "text".to_string() => json!(self.comments),
        "testname".to_string() => json!(self.test_name)
      },
      pending: false,
      plugin_config: self.contents_plugin.as_ref().map(|plugin| {
        hashmap!{
          plugin.name.clone() => self.message_contents.plugin_config.interaction_configuration.clone()
        }
      }).unwrap_or_default(),
      interaction_markup: InteractionMarkup {
        markup: self.message_contents.interaction_markup.clone(),
        markup_type: self.message_contents.interaction_markup_type.clone()
      }
    }
  }

  /// Configure the interaction contents from a map
  pub async fn contents_from<V>(&mut self, contents: HashMap<&str, V>) -> &mut Self
    where V: Clone, Value: From<V> {
    let contents_map: HashMap<String, Value> = contents.iter()
      .map(|(k, v)| {
        (k.to_string(), Value::from(v.clone()))
      })
      .collect();
    debug!("Configuring interaction from {:?}", contents_map);

    if let Some(content_type) = contents_map.get("content-type") {
      let ct = ContentType::parse(json_to_string(content_type).as_str()).unwrap();
      if let Some(content_matcher) = find_content_matcher(&ct) {
        debug!("Found a matcher for '{}': {:?}", ct, content_matcher);
        if content_matcher.is_core() {
          debug!("Content matcher is a core matcher, will use the internal implementation");
          self.setup_core_matcher(&ct, &contents_map, Some(content_matcher));
        } else {
          debug!("Plugin matcher, will get the plugin to provide the interaction contents");
          self.message_contents = content_matcher.configure_interation(&ct, contents_map).await.unwrap();
          self.contents_plugin = content_matcher.plugin();
        }
      } else {
        debug!("No content matcher found, will use the internal implementation");
        self.setup_core_matcher(&ct, &contents_map, None);
      }
    } else {
      self.message_contents = InteractionContents {
        body : OptionalBody::from(Value::Object(contents_map.iter()
          .map(|(k, v)| (k.clone(), v.clone())).collect()).to_string()),
        .. InteractionContents::default()
      };
    }

    self
  }

  fn setup_core_matcher(
    &mut self,
    content_type: &ContentType,
    config: &HashMap<String, Value>,
    content_matcher: Option<ContentMatcher>
  ) {
    self.message_contents = InteractionContents {
      body: if let Some(contents) = config.get("contents") {
        OptionalBody::Present(
          Bytes::from(contents.to_string()),
          Some(content_type.clone()),
          None
        )
      } else {
        OptionalBody::Missing
      },
      .. InteractionContents::default()
    };

    if let Some(content_matcher) = content_matcher {
      // TODO: get the content matcher to apply the matching rules and generators
      //     val (body, rules, generators, _, _) = contentMatcher.setupBodyFromConfig(bodyConfig)
      //     val matchingRules = MatchingRulesImpl()
      //     if (rules != null) {
      //       matchingRules.addCategory(rules)
      //     }
      //     MessageContents(body, mapOf(), matchingRules, generators ?: Generators())
    }
  }

  /// Any global plugin config required to add to the Pact
  pub fn plugin_config(&self) -> Option<PluginData> {
    self.contents_plugin.as_ref().map(|plugin| {
      PluginData {
        name: plugin.name.clone(),
        version: plugin.version.clone(),
        configuration: self.message_contents.plugin_config.pact_configuration.clone()
      }
    })
  }
}

/// Iterator over the messages build with the PactBuilder
pub struct MessageIterator {
  pact: V4Pact,
  message_list: VecDeque<AsynchronousMessage>,
  // Output directory to write pact files to when done
  output_dir: Option<PathBuf>,
}

impl MessageIterator {
  /// Construct a new iterator over the messages in the pact
  pub fn new(pact: V4Pact) -> MessageIterator {
    MessageIterator {
      pact: pact.clone(),
      message_list: pact.filter_interactions(V4InteractionType::Asynchronous_Messages)
        .iter()
        .map(|item| item.as_v4_async_message().unwrap())
        .collect(),
      output_dir: None
    }
  }
}

impl Iterator for MessageIterator {
  type Item = AsynchronousMessage;

  fn next(&mut self) -> Option<Self::Item> {
    self.message_list.pop_front()
  }
}

impl Drop for MessageIterator {
  fn drop(&mut self) {
    if !::std::thread::panicking() {

      dbg!(env::vars().collect::<Vec<(String, String)>>());

      // Write out the Pact file
      let output_dir = self.output_dir.as_ref().map(|dir| dir.to_string_lossy().to_string())
        .unwrap_or_else(|| {
          let val = env::var("PACT_OUTPUT_DIR");
          debug!("env:PACT_OUTPUT_DIR = {:?}", val);
          val.unwrap_or_else(|_| "target/pacts".to_owned())
        });
      let overwrite = env::var("PACT_OVERWRITE");
      debug!("env:PACT_OVERWRITE = {:?}", overwrite);

      let pact_file_name = self.pact.default_file_name();
      let mut path = PathBuf::from(output_dir);
      path.push(pact_file_name);

      info!("Writing pact out to '{}'", path.display());
      let specification = self.pact.specification_version();
      if let Err(err) = write_pact(self.pact.boxed(), path.as_path(), specification,
                                   overwrite.unwrap_or_else(|_| String::default()) == "true") {
        error!("Failed to write pact to file - {}", err);
        panic!("Failed to write pact to file - {}", err);
      }
    }
  }
}