#!/bin/bash -e

cargo clean
cargo build --release
mkdir -p ../target/artifacts
gzip -c ../target/release/pact_ffi.dll > ../target/artifacts/pact_ffi-windows-x86_64.dll.gz
openssl dgst -sha256 -r ../target/artifacts/pact_ffi-windows-x86_64.dll.gz > ../target/artifacts/pact_ffi-windows-x86_64.dll.gz.sha256
gzip -c ../target/release/pact_ffi.dll.lib > ../target/artifacts/pact_ffi-windows-x86_64.dll.lib.gz
openssl dgst -sha256 -r ../target/artifacts/pact_ffi-windows-x86_64.dll.lib.gz > ../target/artifacts/pact_ffi-windows-x86_64.dll.lib.gz.sha256
gzip -c ../target/release/pact_ffi.lib > ../target/artifacts/pact_ffi-windows-x86_64.lib.gz
openssl dgst -sha256 -r ../target/artifacts/pact_ffi-windows-x86_64.lib.gz > ../target/artifacts/pact_ffi-windows-x86_64.lib.gz.sha256


echo -- Build the aarch64 release artifacts --
cargo build --target aarch64-pc-windows-msvc --release
gzip -c ../target/aarch64-pc-windows-msvc/release/pact_ffi.dll > ../target/artifacts/pact_ffi-windows-aarch64.dll.gz
openssl dgst -sha256 -r ../target/artifacts/pact_ffi-windows-aarch64.dll.gz > ../target/artifacts/pact_ffi-windows-aarch64.dll.gz.sha256
gzip -c ../target/aarch64-pc-windows-msvc/release/pact_ffi.dll.lib > ../target/artifacts/pact_ffi-windows-aarch64.dll.lib.gz
openssl dgst -sha256 -r ../target/artifacts/pact_ffi-windows-aarch64.dll.lib.gz > ../target/artifacts/pact_ffi-windows-aarch64.dll.lib.gz.sha256
gzip -c ../target/aarch64-pc-windows-msvc/release/pact_ffi.lib > ../target/artifacts/pact_ffi-windows-aarch64.lib.gz
openssl dgst -sha256 -r ../target/artifacts/pact_ffi-windows-aarch64.lib.gz > ../target/artifacts/pact_ffi-windows-aarch64.lib.gz.sha256