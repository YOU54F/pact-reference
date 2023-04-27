#!/bin/bash

cargo clean
mkdir -p ../target/artifacts
cargo build --release
gzip -c ../target/release/pact_mock_server_cli.exe > ../target/artifacts/pact_mock_server_cli-windows-x86_64.exe.gz
openssl dgst -sha256 -r ../target/artifacts/pact_mock_server_cli-windows-x86_64.exe.gz > ../target/artifacts/pact_mock_server_cli-windows-x86_64.exe.gz.sha256

echo -- Build the aarch64 release artifacts --
## The following is a workaround until ring 16 supports windows arm64 or rustls moves to ring 17 (post release)
## It also relies on ../cargo.toml having the [patch.crates-io] section at the bottom of the file
# https://github.com/briansmith/ring/issues/1514#issuecomment-1258562375
# https://github.com/briansmith/ring/pull/1554
# https://github.com/rust-lang/rustup/issues/2612#issuecomment-1433876793
# https://github.com/rustls/rustls/pull/1108
echo ring = { git = \"https://github.com/awakecoding/ring\", branch = \"0.16.20_alpha\" } >> ../cargo.toml
cd .. && cargo update 
cd pact_mock_server_cli

cargo build --target aarch64-pc-windows-msvc --release
gzip -c ../target/aarch64-pc-windows-msvc/release/pact_mock_server_cli.exe > ../target/artifacts/pact_mock_server_cli-windows-aarch64.exe.gz
openssl dgst -sha256 -r ../target/artifacts/pact_mock_server_cli-windows-aarch64.exe.gz > ../target/artifacts/pact_mock_server_cli-windows-aarch64.exe.gz.sha256