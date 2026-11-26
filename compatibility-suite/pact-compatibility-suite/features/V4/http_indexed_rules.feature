Feature: Matching rules for each value of a header or query parameter
  Headers and query parameters are a list of values for each name. A matching rule key in the header or query
  category can carry a value index, written as a path: $.name[1] or $['name'][1]. An indexed rule applies only to
  that value, and overrides the rule for the name for that value. A key without the index applies to every value.
  The headers column is a JSON object of a name to a list of values, so values can contain commas.

  Scenario: An indexed query rule only applies to its value (positive case)
    Given an expected request configured with the following:
      | query     | matching rules                                                            |
      | a=x&a=100 | JSON: { "query": { "$.a[1]": { "matchers": [{ "match": "regex", "regex": "^[0-9]+$" }] } } } |
    And a request is received with the following:
      | query    |
      | a=x&a=42 |
    When the request is compared to the expected one
    Then the comparison should be OK

  Scenario: An indexed query rule only applies to its value (negative case)
    Given an expected request configured with the following:
      | query     | matching rules                                                            |
      | a=x&a=100 | JSON: { "query": { "$.a[1]": { "matchers": [{ "match": "regex", "regex": "^[0-9]+$" }] } } } |
    And a request is received with the following:
      | query     |
      | a=x&a=abc |
    When the request is compared to the expected one
    Then the comparison should NOT be OK

  Scenario: Values without an indexed query rule are compared for equality
    Given an expected request configured with the following:
      | query     | matching rules                                                            |
      | a=x&a=100 | JSON: { "query": { "$.a[1]": { "matchers": [{ "match": "regex", "regex": "^[0-9]+$" }] } } } |
    And a request is received with the following:
      | query    |
      | a=y&a=42 |
    When the request is compared to the expected one
    Then the comparison should NOT be OK

  Scenario: An indexed query rule overrides the rule for the name for that value (positive case)
    Given an expected request configured with the following:
      | query     | matching rules                                                            |
      | a=x&a=100 | JSON: { "query": { "a": { "matchers": [{ "match": "regex", "regex": "^[a-z]+$" }] }, "$.a[1]": { "matchers": [{ "match": "regex", "regex": "^[0-9]+$" }] } } } |
    And a request is received with the following:
      | query    |
      | a=y&a=42 |
    When the request is compared to the expected one
    Then the comparison should be OK

  Scenario: An indexed query rule overrides the rule for the name for that value (negative case)
    Given an expected request configured with the following:
      | query     | matching rules                                                            |
      | a=x&a=100 | JSON: { "query": { "a": { "matchers": [{ "match": "regex", "regex": "^[a-z]+$" }] }, "$.a[1]": { "matchers": [{ "match": "regex", "regex": "^[0-9]+$" }] } } } |
    And a request is received with the following:
      | query   |
      | a=y&a=z |
    When the request is compared to the expected one
    Then the comparison should NOT be OK

  Scenario: A query parameter name with brackets is a plain name
    Given an expected request configured with the following:
      | query       | matching rules                                                              |
      | ids[0]=100  | JSON: { "query": { "ids[0]": { "matchers": [{ "match": "regex", "regex": "^[0-9]+$" }] } } } |
    And a request is received with the following:
      | query     |
      | ids[0]=42 |
    When the request is compared to the expected one
    Then the comparison should be OK

  Scenario: An indexed header rule only applies to its value (positive case)
    Given an expected request configured with the following:
      | headers                         | matching rules                                                                    |
      | { "X-Request": ["a", "100"] }   | JSON: { "header": { "$['X-Request'][1]": { "matchers": [{ "match": "regex", "regex": "^[0-9]+$" }] } } } |
    And a request is received with the following:
      | headers                      |
      | { "X-Request": ["a", "42"] } |
    When the request is compared to the expected one
    Then the comparison should be OK

  Scenario: An indexed header rule only applies to its value (negative case)
    Given an expected request configured with the following:
      | headers                         | matching rules                                                                    |
      | { "X-Request": ["a", "100"] }   | JSON: { "header": { "$['X-Request'][1]": { "matchers": [{ "match": "regex", "regex": "^[0-9]+$" }] } } } |
    And a request is received with the following:
      | headers                       |
      | { "X-Request": ["a", "abc"] } |
    When the request is compared to the expected one
    Then the comparison should NOT be OK

  Scenario: An indexed header rule overrides the rule for the name for that value
    Given an expected request configured with the following:
      | headers                         | matching rules                                                                    |
      | { "X-Request": ["a", "100"] }   | JSON: { "header": { "X-Request": { "matchers": [{ "match": "regex", "regex": "^[a-z]+$" }] }, "$['X-Request'][1]": { "matchers": [{ "match": "regex", "regex": "^[0-9]+$" }] } } } |
    And a request is received with the following:
      | headers                      |
      | { "X-Request": ["b", "42"] } |
    When the request is compared to the expected one
    Then the comparison should be OK

  Scenario: Header names in indexed rule keys are not case sensitive
    Given an expected request configured with the following:
      | headers                         | matching rules                                                                    |
      | { "X-Request": ["a", "100"] }   | JSON: { "header": { "$['x-request'][1]": { "matchers": [{ "match": "regex", "regex": "^[0-9]+$" }] } } } |
    And a request is received with the following:
      | headers                      |
      | { "x-request": ["a", "42"] } |
    When the request is compared to the expected one
    Then the comparison should be OK

  Scenario: Header values can contain commas
    Given an expected request configured with the following:
      | headers                                          | matching rules                                                                    |
      | { "X-Request": ["a", "Tue, 01 Jan 2030 10:00:00 GMT"] } | JSON: { "header": { "$['X-Request'][1]": { "matchers": [{ "match": "regex", "regex": "^[A-Z][a-z]{2}, .* GMT$" }] } } } |
    And a request is received with the following:
      | headers                                                 |
      | { "X-Request": ["a", "Wed, 02 Jan 2030 11:00:00 GMT"] } |
    When the request is compared to the expected one
    Then the comparison should be OK
