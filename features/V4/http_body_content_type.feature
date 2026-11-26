Feature: V4 body content type without a Content-Type header
  A V4 body records its own content type, so an interaction can have a typed body without a Content-Type header.
  The body content type decides how the body is compared, even when the received request has no Content-Type header
  and its body would be sniffed as something else. The Content-Type header is only an expectation when it is in the
  interaction's headers.

  Scenario: Typed body, request sent with no Content-Type header
    Given an expected request configured with the following:
      | body             | content type             |
      | file: sample.pdf | application/octet-stream |
    And the "content-type" header is removed from the expected request
    And a request is received with the following:
      | body             |
      | file: sample.pdf |
    And the "content-type" header is removed from the received request
    When the request is compared to the expected one
    Then the comparison should be OK

  Scenario: Typed body, request sent with a different body and no Content-Type header
    Given an expected request configured with the following:
      | body             | content type             |
      | file: sample.pdf | application/octet-stream |
    And the "content-type" header is removed from the expected request
    And a request is received with the following:
      | body          |
      | file: rat.jpg |
    And the "content-type" header is removed from the received request
    When the request is compared to the expected one
    Then the comparison should NOT be OK

  Scenario: Typed body, request sent with a matching Content-Type header
    Given an expected request configured with the following:
      | body             | content type             |
      | file: sample.pdf | application/octet-stream |
    And the "content-type" header is removed from the expected request
    And a request is received with the following:
      | body             | content type             |
      | file: sample.pdf | application/octet-stream |
    When the request is compared to the expected one
    Then the comparison should be OK

  Scenario: Typed body, request sent with a Content-Type header that declares a different type
    Given an expected request configured with the following:
      | body             | content type             |
      | file: sample.pdf | application/octet-stream |
    And the "content-type" header is removed from the expected request
    And a request is received with the following:
      | body             | content type     |
      | file: sample.pdf | application/json |
    When the request is compared to the expected one
    Then the comparison should NOT be OK

  Scenario: The Content-Type header is still required when it is in the interaction
    Given an expected request configured with the following:
      | body             | content type             |
      | file: sample.pdf | application/octet-stream |
    And a request is received with the following:
      | body             |
      | file: sample.pdf |
    And the "content-type" header is removed from the received request
    When the request is compared to the expected one
    Then the comparison should NOT be OK
