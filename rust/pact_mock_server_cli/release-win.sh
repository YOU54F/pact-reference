#!/bin/bash

cargo clean
mkdir -p ../target/artifacts
cargo build --release
gzip -c ../target/release/pact_mock_server_cli.exe > ../target/artifacts/pact_mock_server_cli-windows-x86_64.exe.gz
openssl dgst -sha256 -r ../target/artifacts/pact_mock_server_cli-windows-x86_64.exe.gz > ../target/artifacts/pact_mock_server_cli-windows-x86_64.exe.gz.sha256

echo -- Build the aarch64 release artifacts --

cargo build --target aarch64-pc-windows-msvc --release
gzip -c ../target/aarch64-pc-windows-msvc/release/pact_mock_server_cli.exe > ../target/artifacts/pact_mock_server_cli-windows-aarch64.exe.gz
openssl dgst -sha256 -r ../target/artifacts/pact_mock_server_cli-windows-aarch64.exe.gz > ../target/artifacts/pact_mock_server_cli-windows-aarch64.exe.gz.sha256