//! Traits and structs for dealing with the test context.

use std::panic::RefUnwindSafe;

use itertools::Itertools;

use pact_models::matchingrules::{MatchingRuleCategory, RuleList};
use pact_models::path_exp::DocPath;
use pact_models::prelude::v4::{SynchronousHttp, V4Pact};
use pact_models::v4::interaction::V4Interaction;

/// Configuration for driving behaviour of the execution
#[derive(Copy, Clone, Debug)]
pub struct MatchingConfiguration {
  /// If extra keys/values are allowed (and ignored)
  pub allow_unexpected_entries: bool,
  /// If the executed plan should be logged
  pub log_executed_plan: bool,
  /// If the executed plan summary should be logged
  pub log_plan_summary: bool,
  /// If output should be coloured
  pub coloured_output: bool
}

impl MatchingConfiguration {
  /// Configures the matching engine configuration from environment variables:
  /// * `V2_MATCHING_LOG_EXECUTED_PLAN` - Enable to log the executed plan.
  /// * `V2_MATCHING_LOG_PLAN_SUMMARY` - Enable to log a summary of the executed plan.
  /// * `V2_MATCHING_COLOURED_OUTPUT` - Enables coloured output.
  pub fn init_from_env() -> Self {
    let mut config = MatchingConfiguration::default();

    if let Some(val) = env_var_set("V2_MATCHING_LOG_EXECUTED_PLAN") {
      config.log_executed_plan = val;
    }
    if let Some(val) = env_var_set("V2_MATCHING_LOG_PLAN_SUMMARY") {
      config.log_plan_summary = val;
    }
    if let Some(val) = env_var_set("V2_MATCHING_COLOURED_OUTPUT") {
      config.coloured_output = val;
    }

    config
  }
}

fn env_var_set(name: &str) -> Option<bool> {
  std::env::var(name)
    .ok()
    .map(|v| ["true", "1"].contains(&v.to_lowercase().as_str()))
}

impl Default for MatchingConfiguration {
  fn default() -> Self {
    MatchingConfiguration {
      allow_unexpected_entries: false,
      log_executed_plan: false,
      log_plan_summary: true,
      coloured_output: true
    }
  }
}

/// Context to store data for use in executing an execution plan.
#[derive(Clone, Debug)]
pub struct PlanMatchingContext {
  /// Pact the plan is for
  pub pact: V4Pact,
  /// Interaction that the plan id for
  pub interaction: Box<dyn V4Interaction + Send + Sync + RefUnwindSafe>,
  /// Matching rules to use
  pub matching_rules: MatchingRuleCategory,
  /// Configuration
  pub config: MatchingConfiguration
}

impl Default for PlanMatchingContext {
  fn default() -> Self {
    PlanMatchingContext {
      pact: Default::default(),
      interaction: Box::new(SynchronousHttp::default()),
      matching_rules: Default::default(),
      config: Default::default()
    }
  }
}

impl PlanMatchingContext {
  /// If there is a matcher defined at the path in this context
  pub fn matcher_is_defined(&self, path: &DocPath) -> bool {
    let path = path.to_vec();
    let path_slice = path.iter().map(|p| p.as_str()).collect_vec();
    self.matching_rules.matcher_is_defined(path_slice.as_slice())
  }

  /// Select the best matcher to use for the given path
  pub fn select_best_matcher(&self, path: &DocPath) -> RuleList {
    let path = path.to_vec();
    let path_slice = path.iter().map(|p| p.as_str()).collect_vec();
    self.matching_rules.select_best_matcher(path_slice.as_slice())
  }

  /// Creates a clone of this context, but with the matching rules set for the Request Method
  pub fn for_method(&self) -> Self {
    let matching_rules = if let Some(req_res) = self.interaction.as_v4_http() {
      req_res.request.matching_rules.rules_for_category("method").unwrap_or_default()
    } else {
      MatchingRuleCategory::default()
    };

    PlanMatchingContext {
      pact: self.pact.clone(),
      interaction: self.interaction.boxed_v4(),
      matching_rules,
      config: self.config
    }
  }

  /// Creates a clone of this context, but with the matching rules set for the Request Path
  pub fn for_path(&self) -> Self {
    let matching_rules = if let Some(req_res) = self.interaction.as_v4_http() {
      req_res.request.matching_rules.rules_for_category("path").unwrap_or_default()
    } else {
      MatchingRuleCategory::default()
    };

    PlanMatchingContext {
      pact: self.pact.clone(),
      interaction: self.interaction.boxed_v4(),
      matching_rules,
      config: self.config
    }
  }

  /// Creates a clone of this context, but with the matching rules set for the Request Query Parameters
  pub fn for_query(&self) -> Self {
    let matching_rules = if let Some(req_res) = self.interaction.as_v4_http() {
      req_res.request.matching_rules.rules_for_category("query").unwrap_or_default()
    } else {
      MatchingRuleCategory::default()
    };

    PlanMatchingContext {
      pact: self.pact.clone(),
      interaction: self.interaction.boxed_v4(),
      matching_rules,
      config: self.config
    }
  }

  /// Creates a clone of this context, but with the matching rules set for the Request Headers
  pub fn for_headers(&self) -> Self {
    let matching_rules = if let Some(req_res) = self.interaction.as_v4_http() {
      req_res.request.matching_rules.rules_for_category("header").unwrap_or_default()
    } else {
      MatchingRuleCategory::default()
    };

    PlanMatchingContext {
      pact: self.pact.clone(),
      interaction: self.interaction.boxed_v4(),
      matching_rules,
      config: self.config
    }
  }

  /// Creates a clone of this context, but with the matching rules set for the Request Body
  pub fn for_body(&self) -> Self {
    let matching_rules = if let Some(req_res) = self.interaction.as_v4_http() {
      req_res.request.matching_rules.rules_for_category("body").unwrap_or_default()
    } else {
      MatchingRuleCategory::default()
    };

    PlanMatchingContext {
      pact: self.pact.clone(),
      interaction: self.interaction.boxed_v4(),
      matching_rules,
      config: self.config
    }
  }
}