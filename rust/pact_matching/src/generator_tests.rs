use std::str::FromStr;

use expectest::expect;
use expectest::prelude::*;
use pact_models::bodies::OptionalBody;
use pact_models::content_types::JSON;
use pact_models::generators;
use pact_models::generators::{ContentTypeHandler, Generator, JsonHandler};
use pact_models::message::Message;
use pact_models::path_exp::DocPath;
use serde_json::Value;

use crate::generators::generate_message;

use super::*;

#[tokio::test]
async fn returns_original_response_if_there_are_no_generators() {
  let response = HttpResponse::default();
  expect!(generate_response(&response, &GeneratorTestMode::Provider, &hashmap!{}).await).to(be_equal_to(response));
}

#[tokio::test]
async fn applies_status_generator_for_status_to_the_copy_of_the_response() {
  let response = HttpResponse { status: 200, generators: generators! {
    "STATUS" => Generator::RandomInt(400, 499)
  }, .. HttpResponse::default() };
  expect!(generate_response(&response, &GeneratorTestMode::Provider, &hashmap!{}).await.status).to(be_greater_or_equal_to(400));
}

#[tokio::test]
async fn applies_header_generator_for_headers_to_the_copy_of_the_response() {
  let response = HttpResponse { headers: Some(hashmap!{
      s!("A") => vec![s!("a")],
      s!("B") => vec![s!("b")]
    }), generators: generators! {
      "HEADER" => {
        "A" => Generator::Uuid(None)
      }
    }, .. HttpResponse::default()
  };
  let headers = generate_response(&response, &GeneratorTestMode::Provider, &hashmap!{}).await.headers.unwrap().clone();
  expect!(headers.get("A").unwrap().first().unwrap()).to_not(be_equal_to("a"));
}

#[tokio::test]
async fn returns_original_request_if_there_are_no_generators() {
  let request = HttpRequest::default();
  expect!(generate_request(&request, &GeneratorTestMode::Provider, &hashmap!{}).await).to(be_equal_to(request));
}

#[tokio::test]
async fn applies_path_generator_for_the_path_to_the_copy_of_the_request() {
  let request = HttpRequest { path: s!("/path"), generators: generators! {
    "PATH" => Generator::RandomInt(1, 10)
  }, .. HttpRequest::default() };
  expect!(generate_request(&request, &GeneratorTestMode::Provider, &hashmap!{}).await.path).to_not(be_equal_to("/path"));
}

#[tokio::test]
async fn applies_header_generator_for_headers_to_the_copy_of_the_request() {
  let request = HttpRequest { headers: Some(hashmap!{
      s!("A") => vec![s!("a")],
      s!("B") => vec![s!("b")]
    }), generators: generators! {
      "HEADER" => {
        "A" => Generator::Uuid(None)
      }
    }, .. HttpRequest::default()
  };
  let headers = generate_request(&request, &GeneratorTestMode::Provider, &hashmap!{}).await.headers.unwrap().clone();
  expect!(headers.get("A").unwrap().first().unwrap()).to_not(be_equal_to("a"));
}

#[tokio::test]
async fn applies_query_generator_for_query_parameters_to_the_copy_of_the_request() {
  let request = HttpRequest { query: Some(hashmap!{
      "A".to_string() => vec![ Some("a".to_string()) ],
      "B".to_string() => vec![ Some("b".to_string()) ]
    }), generators: generators! {
      "QUERY" => {
        "A" => Generator::Uuid(None)
      }
    }, .. HttpRequest::default()
  };
  let query = generate_request(&request, &GeneratorTestMode::Provider, &hashmap!{}).await.query.unwrap().clone();
  let query_val = query.get("A").unwrap()[0].as_ref();
  expect!(query_val.unwrap()).to_not(be_equal_to("a"));
}

#[test_log::test(tokio::test)]
async fn applies_provider_state_generator_for_query_parameters_with_square_brackets() {
  let request = HttpRequest {
    query: Some(hashmap!{
      "A".to_string() => vec![ Some("a".to_string()) ],
      "q[]".to_string() => vec![ Some("q1".to_string()), Some("q2".to_string()) ]
    }),
    generators: generators! {
      "QUERY" => {
        "A" => Generator::ProviderStateGenerator("exp1".to_string(), None),
        "$['q[]']" => Generator::ProviderStateGenerator("${exp2}".to_string(), None)
      }
    }, .. HttpRequest::default()
  };
  let context = hashmap! {
    "exp1" => json!("1234"),
    "exp2" => json!("5678")
  };
  let result = generate_request(&request, &GeneratorTestMode::Provider, &context).await;
  let query = result.query.unwrap();
  let a_val = query.get("A").unwrap();
  expect!(a_val).to(be_equal_to(&vec![Some("1234".to_string())]));
  let q_val = query.get("q[]").unwrap();
  expect!(q_val).to(be_equal_to(&vec![Some("5678".to_string()), Some("5678".to_string())]));
}

#[tokio::test]
async fn header_generator_for_the_name_applies_to_each_value_and_ignores_case() {
  let request = HttpRequest { headers: Some(hashmap!{
      s!("X-Request") => vec![s!("a"), s!("b")]
    }), generators: generators! {
      "HEADER" => {
        "x-request" => Generator::ProviderStateGenerator("exp".to_string(), None)
      }
    }, .. HttpRequest::default()
  };
  let context = hashmap! { "exp" => json!("Z") };
  let headers = generate_request(&request, &GeneratorTestMode::Provider, &context).await.headers.unwrap();
  expect!(headers.get("X-Request").unwrap()).to(be_equal_to(&vec![s!("Z"), s!("Z")]));
}

#[tokio::test]
async fn header_generator_for_a_value_index_applies_only_to_that_value() {
  let response = HttpResponse { headers: Some(hashmap!{
      s!("X-Request") => vec![s!("a"), s!("b"), s!("c")]
    }), generators: generators! {
      "HEADER" => {
        "$['X-Request'][1]" => Generator::ProviderStateGenerator("exp".to_string(), None),
        "$['x-request'][5]" => Generator::ProviderStateGenerator("exp".to_string(), None)
      }
    }, .. HttpResponse::default()
  };
  let context = hashmap! { "exp" => json!("Z") };
  let headers = generate_response(&response, &GeneratorTestMode::Provider, &context).await.headers.unwrap();
  expect!(headers.get("X-Request").unwrap()).to(be_equal_to(&vec![s!("a"), s!("Z"), s!("c")]));
}

#[tokio::test]
async fn header_generator_for_a_value_index_runs_after_the_one_for_the_name() {
  let request = HttpRequest { headers: Some(hashmap!{
      s!("X-Request") => vec![s!("a"), s!("b")]
    }), generators: generators! {
      "HEADER" => {
        "$['X-Request'][0]" => Generator::ProviderStateGenerator("first".to_string(), None),
        "X-Request" => Generator::ProviderStateGenerator("all".to_string(), None)
      }
    }, .. HttpRequest::default()
  };
  let context = hashmap! { "first" => json!("B"), "all" => json!("A") };
  let headers = generate_request(&request, &GeneratorTestMode::Provider, &context).await.headers.unwrap();
  expect!(headers.get("X-Request").unwrap()).to(be_equal_to(&vec![s!("B"), s!("A")]));
}

#[tokio::test]
async fn header_generator_for_a_missing_header_generates_a_single_value() {
  let request = HttpRequest { generators: generators! {
      "HEADER" => {
        "X-Id" => Generator::ProviderStateGenerator("exp".to_string(), None),
        "$['X-Other'][1]" => Generator::ProviderStateGenerator("exp".to_string(), None)
      }
    }, .. HttpRequest::default()
  };
  let context = hashmap! { "exp" => json!("Z") };
  let headers = generate_request(&request, &GeneratorTestMode::Provider, &context).await.headers.unwrap();
  expect!(headers).to(be_equal_to(hashmap!{ s!("X-Id") => vec![s!("Z")] }));
}

#[tokio::test]
async fn query_generator_for_a_missing_parameter_generates_a_single_value() {
  let request = HttpRequest { query: Some(hashmap!{
      "a".to_string() => vec![ Some("1".to_string()) ]
    }), generators: generators! {
      "QUERY" => {
        "id" => Generator::ProviderStateGenerator("exp".to_string(), None),
        "$.other[1]" => Generator::ProviderStateGenerator("exp".to_string(), None)
      }
    }, .. HttpRequest::default()
  };
  let context = hashmap! { "exp" => json!("Z") };
  let query = generate_request(&request, &GeneratorTestMode::Provider, &context).await.query.unwrap();
  expect!(query).to(be_equal_to(hashmap!{
    s!("a") => vec![Some(s!("1"))],
    s!("id") => vec![Some(s!("Z"))]
  }));
}

#[tokio::test]
async fn query_generator_for_a_value_index_applies_only_to_that_value() {
  // A key without the leading `$` is a plain parameter name, as when it is loaded from a pact file
  let mut plain_name = DocPath::root();
  plain_name.push_field("ids[0]");
  let mut request = HttpRequest { query: Some(hashmap!{
      "ids".to_string() => vec![ Some("1".to_string()), Some("2".to_string()) ],
      "ids[0]".to_string() => vec![ Some("x".to_string()) ]
    }), generators: generators! {
      "QUERY" => {
        "$.ids[1]" => Generator::ProviderStateGenerator("exp".to_string(), None)
      }
    }, .. HttpRequest::default()
  };
  request.generators.add_generator_with_subcategory(&GeneratorCategory::QUERY, plain_name,
    Generator::ProviderStateGenerator("exp".to_string(), None));
  let context = hashmap! { "exp" => json!("Z") };
  let query = generate_request(&request, &GeneratorTestMode::Provider, &context).await.query.unwrap();
  expect!(query.get("ids").unwrap()).to(be_equal_to(&vec![Some(s!("1")), Some(s!("Z"))]));
  expect!(query.get("ids[0]").unwrap()).to(be_equal_to(&vec![Some(s!("Z"))]));
}

#[tokio::test]
async fn applies_body_generator_to_the_copy_of_the_request() {
  let request = HttpRequest { body: OptionalBody::Present("{\"a\": 100, \"b\": \"B\"}".into(), None, None),
    generators: generators! {
      "BODY" => {
        "$.a" => Generator::RandomInt(1, 10)
      }
    }, .. HttpRequest::default()
  };
  let generated_request = generate_request(&request, &GeneratorTestMode::Provider, &hashmap!{}).await;
  let body: Value = serde_json::from_str(generated_request.body.display_string().as_str()).unwrap();
  expect!(&body["a"]).to_not(be_equal_to(&json!(100)));
  expect!(&body["b"]).to(be_equal_to(&json!("B")));
}

#[tokio::test]
async fn applies_body_generator_to_the_copy_of_the_response() {
  let response = HttpResponse { body: OptionalBody::Present("{\"a\": 100, \"b\": \"B\"}".into(), None, None),
    generators: generators! {
      "BODY" => {
        "$.a" => Generator::RandomInt(1, 10)
      }
    }, .. HttpResponse::default()
  };
  let body: Value = serde_json::from_str(generate_response(&response,
   &GeneratorTestMode::Provider, &hashmap!{}).await.body.display_string().as_str()).unwrap();
  expect!(&body["a"]).to_not(be_equal_to(&json!(100)));
  expect!(&body["b"]).to(be_equal_to(&json!("B")));
}

#[test]
fn applies_the_generator_to_a_json_map_entry() {
  let map = json!({"a": 100, "b": "B", "c": "C"});
  let mut json_handler = JsonHandler { value: map };

  json_handler.apply_key(&DocPath::new_unwrap("$.b"), &Generator::RandomInt(0, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(&json_handler.value["b"]).to_not(be_equal_to(&json!("B")));
}

#[test]
fn does_not_apply_the_generator_when_field_is_not_in_map() {
  let map = json!({"a": 100, "b": "B", "c": "C"});
  let mut json_handler = JsonHandler { value: map };

  json_handler.apply_key(&DocPath::new_unwrap("$.d"), &Generator::RandomInt(0, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(json_handler.value).to(be_equal_to(json!({"a": 100, "b": "B", "c": "C"})));
}

#[test]
fn does_not_apply_the_generator_when_not_a_map() {
  let map = json!(100);
  let mut json_handler = JsonHandler { value: map };

  json_handler.apply_key(&DocPath::new_unwrap("$.d"), &Generator::RandomInt(0, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(json_handler.value).to(be_equal_to(json!(100)));
}

#[test]
fn applies_the_generator_to_a_list_item() {
  let list = json!([100, 200, 300]);
  let mut json_handler = JsonHandler { value: list };

  json_handler.apply_key(&DocPath::new_unwrap("$[1]"), &Generator::RandomInt(0, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(&json_handler.value[1]).to_not(be_equal_to(&json!(200)));
}

#[test]
fn does_not_apply_the_generator_when_index_is_not_in_list() {
  let list = json!([100, 200, 300]);
  let mut json_handler = JsonHandler { value: list };

  json_handler.apply_key(&DocPath::new_unwrap("$[3]"), &Generator::RandomInt(0, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(json_handler.value).to(be_equal_to(json!([100, 200, 300])));
}

#[test]
fn does_not_apply_the_generator_when_not_a_list() {
  let list = json!(100);
  let mut json_handler = JsonHandler { value: list };

  json_handler.apply_key(&DocPath::new_unwrap("$[3]"), &Generator::RandomInt(0, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(json_handler.value).to(be_equal_to(json!(100)));
}

#[test]
fn applies_the_generator_to_the_root() {
  let value = json!(100);
  let mut json_handler = JsonHandler { value };

  json_handler.apply_key(&DocPath::root(), &Generator::RandomInt(0, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(&json_handler.value).to_not(be_equal_to(&json!(100)));
}

#[test]
fn applies_the_generator_to_the_object_graph() {
  let value = json!({
    "a": ["A", {"a": "A", "b": {"1": "1", "2": "2"}, "c": "C"}, "C"],
    "b": "B",
    "c": "C"
  });
  let mut json_handler = JsonHandler { value };

  json_handler.apply_key(&DocPath::new_unwrap("$.a[1].b['2']"), &Generator::RandomInt(3, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(&json_handler.value["a"][1]["b"]["2"]).to_not(be_equal_to(&json!("2")));
}

#[test]
fn does_not_apply_the_generator_to_the_object_graph_when_the_expression_does_not_match() {
  let value = json!({
    "a": "A",
    "b": "B",
    "c": "C"
  });
  let mut json_handler = JsonHandler { value };

  json_handler.apply_key(&DocPath::new_unwrap("$.a[1].b['2']"), &Generator::RandomInt(0, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(&json_handler.value).to(be_equal_to(&json!({
    "a": "A",
    "b": "B",
    "c": "C"
  })));
}

#[test]
fn applies_the_generator_to_all_map_entries() {
  let value = json!({
    "a": "A",
    "b": "B",
    "c": "C"
  });
  let mut json_handler = JsonHandler { value };

  json_handler.apply_key(&DocPath::new_unwrap("$.*"), &Generator::RandomInt(0, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(&json_handler.value["a"]).to_not(be_equal_to(&json!("A")));
  expect!(&json_handler.value["b"]).to_not(be_equal_to(&json!("B")));
  expect!(&json_handler.value["c"]).to_not(be_equal_to(&json!("C")));
}

#[test]
fn applies_the_generator_to_all_list_items() {
  let value = json!(["A", "B", "C"]);
  let mut json_handler = JsonHandler { value };

  json_handler.apply_key(&DocPath::new_unwrap("$[*]"), &Generator::RandomInt(0, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(&json_handler.value[0]).to_not(be_equal_to(&json!("A")));
  expect!(&json_handler.value[1]).to_not(be_equal_to(&json!("B")));
  expect!(&json_handler.value[2]).to_not(be_equal_to(&json!("C")));
}

#[test]
fn applies_the_generator_to_the_object_graph_with_wildcard() {
  let value = json!({
    "a": ["A", {"a": "A", "b": ["1", "2"], "c": "C"}, "C"],
    "b": "B",
    "c": "C"
  });
  let mut json_handler = JsonHandler { value };

  json_handler.apply_key(&DocPath::new_unwrap("$.*[1].b[*]"), &Generator::RandomInt(3, 10), &hashmap!{}, &DefaultVariantMatcher.boxed());

  expect!(&json_handler.value["a"][0]).to(be_equal_to(&json!("A")));
  expect!(&json_handler.value["a"][1]["a"]).to(be_equal_to(&json!("A")));
  expect!(&json_handler.value["a"][1]["b"][0]).to_not(be_equal_to(&json!("1")));
  expect!(&json_handler.value["a"][1]["b"][1]).to_not(be_equal_to(&json!("2")));
  expect!(&json_handler.value["a"][1]["c"]).to(be_equal_to(&json!("C")));
  expect!(&json_handler.value["a"][2]).to(be_equal_to(&json!("C")));
  expect!(&json_handler.value["b"]).to(be_equal_to(&json!("B")));
  expect!(&json_handler.value["c"]).to(be_equal_to(&json!("C")));
}

#[tokio::test]
async fn returns_original_message_if_there_are_no_generators() {
  let message = Message::default();
  expect!(generate_message(&message, &GeneratorTestMode::Provider, &hashmap!{}, &vec![], &hashmap!{}).await).to(be_equal_to(message));
}

#[tokio::test]
async fn applies_metadata_generator_for_to_the_copy_of_the_message() {
  let message = Message {
    metadata: hashmap!{
      "A".to_string() => json!("a"),
      "B".to_string() => json!("b")
    },
    generators: generators! {
      "METADATA" => {
        "A" => Generator::Uuid(None)
      }
    }, ..  Message::default()
  };
  let generated = generate_message(&message, &GeneratorTestMode::Provider, &hashmap!{}, &vec![], &hashmap!{}).await;
  expect!(generated.metadata.get("A").unwrap()).to_not(be_equal_to(&json!("a")));
}

#[tokio::test]
async fn applies_body_generator_to_the_copy_of_the_message() {
  let message = Message {
    contents: OptionalBody::Present("{\"a\": 100, \"b\": \"B\"}".into(), Some(JSON.clone()), None),
    generators: generators! {
      "BODY" => {
        "$.a" => Generator::RandomInt(1, 10)
      }
    }, ..  Message::default()
  };
  let generated = generate_message(&message, &GeneratorTestMode::Provider, &hashmap!{}, &vec![], &hashmap!{}).await;
  let json_str = generated.contents.value_as_string().unwrap();
  let body: Value = serde_json::from_str(json_str.as_str()).unwrap();
  expect!(&body["a"]).to_not(be_equal_to(&json!(100)));
  expect!(&body["b"]).to(be_equal_to(&json!("B")));
}
