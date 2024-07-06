SKIP_SIGNAL_HANDLERS?=false
TEST_COUNT?=1
RACE=-race
CGO_ENABLED?=1
export DYLD_LIBRARY_PATH=/Users/yousaf.nabi/dev/pact-foundation/pact-reference/rust/target/release
export CGO_LDFLAGS=-L/Users/yousaf.nabi/dev/pact-foundation/pact-reference/rust/target/release
ifeq ($(CGO_ENABLED),0)
	SKIP_RACE=
endif
ifeq ($(SKIP_RACE),true)
	RACE=
endif
# export PACT_PROVIDER_DIR?=./pacts_plugin_protobuf

ifeq ($(PACT_PROVIDER),protobuf)
	export PACT_PROVIDER_DIR=./pacts_plugin_protobuf
else ifeq ($(PACT_PROVIDER),csv)
	export PACT_PROVIDER_DIR=./pacts_plugin_csv
else ifeq ($(PACT_PROVIDER),avro)
	export PACT_PROVIDER_DIR=./pacts_plugin_avro
else ifeq ($(PACT_PROVIDER),go)
	export PACT_PROVIDER_DIR=./pacts_plugin_go
else ifeq ($(PACT_PROVIDER),message)
	export PACT_PROVIDER_DIR=./pacts_plugin_message
else
	export PACT_PROVIDER_DIR=./pacts_http
endif

run:
	go run pact/go/native/native

test:
	go test -x -v $(SKIP_RACE) -count $(TEST_COUNT) pact/go/native/native

test_protobuf:
	make test PACT_PROVIDER=protobuf
test_csv:
	make test PACT_PROVIDER=csv
test_avro:
	make test PACT_PROVIDER=avro
test_go:
	make test PACT_PROVIDER=go
test_message:
	make test PACT_PROVIDER=message
test_http:
	make test