Feature: Generators for each value of a header or query parameter
  A generator key in the header or query category can carry a value index, written as a path: $.name[1] or
  $['name'][1]. An indexed generator applies only to that value, and runs after the generator for the name. A key
  without the index applies to every value, and adds the header or query parameter with a single generated value if
  it is not there. Header names are matched without regard to case. A key without the leading $ is a plain name, so
  ids[0] is the parameter named "ids[0]". The headers column is a JSON object of a name to a list of values, and the
  expected values are a JSON list.

  Background:
    Given the generator test mode is set as "Provider"

  Scenario: A query generator for the name applies to every value
    Given a request configured with the following generators:
      | query   | generators                                                                    |
      | a=1&a=2 | JSON: { "query": { "a": { "type": "ProviderState", "expression": "${id}" } } } |
    When the request is prepared for use with a "providerState" context:
      | { "id": "X" } |
    Then the request query parameter "a" will be ["X", "X"]

  Scenario: An indexed query generator only applies to its value
    Given a request configured with the following generators:
      | query       | generators                                                                         |
      | a=1&a=2&a=3 | JSON: { "query": { "$.a[1]": { "type": "ProviderState", "expression": "${id}" } } } |
    When the request is prepared for use with a "providerState" context:
      | { "id": "X" } |
    Then the request query parameter "a" will be ["1", "X", "3"]

  Scenario: An indexed query generator runs after the generator for the name
    Given a request configured with the following generators:
      | query   | generators                                                                                                                                       |
      | a=1&a=2 | JSON: { "query": { "$.a[0]": { "type": "ProviderState", "expression": "${first}" }, "a": { "type": "ProviderState", "expression": "${all}" } } } |
    When the request is prepared for use with a "providerState" context:
      | { "first": "F", "all": "A" } |
    Then the request query parameter "a" will be ["F", "A"]

  Scenario: An indexed query generator for a value that is not there is ignored
    Given a request configured with the following generators:
      | query | generators                                                                         |
      | a=1   | JSON: { "query": { "$.a[3]": { "type": "ProviderState", "expression": "${id}" } } } |
    When the request is prepared for use with a "providerState" context:
      | { "id": "X" } |
    Then the request query parameter "a" will be ["1"]

  Scenario: A query generator key without the leading $ is a plain parameter name
    Given a request configured with the following generators:
      | query           | generators                                                                         |
      | ids=1&ids[0]=2  | JSON: { "query": { "ids[0]": { "type": "ProviderState", "expression": "${id}" } } } |
    When the request is prepared for use with a "providerState" context:
      | { "id": "X" } |
    Then the request query parameter "ids" will be ["1"]
    And the request query parameter "ids[0]" will be ["X"]

  Scenario: A query generator for a parameter that is not there generates a single value
    Given a request configured with the following generators:
      | query | generators                                                                     |
      | a=1   | JSON: { "query": { "id": { "type": "ProviderState", "expression": "${id}" } } } |
    When the request is prepared for use with a "providerState" context:
      | { "id": "X" } |
    Then the request query parameter "a" will be ["1"]
    And the request query parameter "id" will be ["X"]

  Scenario: A header generator for the name applies to every value
    Given a request configured with the following generators:
      | headers                       | generators                                                                              |
      | { "X-Request": ["a", "b"] }   | JSON: { "header": { "X-Request": { "type": "ProviderState", "expression": "${id}" } } } |
    When the request is prepared for use with a "providerState" context:
      | { "id": "X" } |
    Then the request header "X-Request" will be ["X", "X"]

  Scenario: An indexed header generator only applies to its value
    Given a request configured with the following generators:
      | headers                          | generators                                                                                     |
      | { "X-Request": ["a", "b", "c"] } | JSON: { "header": { "$['X-Request'][1]": { "type": "ProviderState", "expression": "${id}" } } } |
    When the request is prepared for use with a "providerState" context:
      | { "id": "X" } |
    Then the request header "X-Request" will be ["a", "X", "c"]

  Scenario: Header generators match the header name without regard to case
    Given a request configured with the following generators:
      | headers                     | generators                                                                                                                                                      |
      | { "X-Request": ["a", "b"] } | JSON: { "header": { "x-request": { "type": "ProviderState", "expression": "${all}" }, "$['X-REQUEST'][1]": { "type": "ProviderState", "expression": "${second}" } } } |
    When the request is prepared for use with a "providerState" context:
      | { "all": "A", "second": "S" } |
    Then the request header "X-Request" will be ["A", "S"]

  Scenario: A header generator for a header that is not there generates a single value
    Given a request configured with the following generators:
      | generators                                                                         |
      | JSON: { "header": { "X-Id": { "type": "ProviderState", "expression": "${id}" } } } |
    When the request is prepared for use with a "providerState" context:
      | { "id": "X" } |
    Then the request header "X-Id" will be ["X"]
