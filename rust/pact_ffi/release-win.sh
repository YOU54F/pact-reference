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

## The following is a workaround until ring 16 supports windows arm64 or rustls moves to ring 17 (post release)
## It also relies on ../cargo.toml having the [patch.crates-io] section at the bottom of the file
# https://github.com/briansmith/ring/issues/1514#issuecomment-1258562375
# https://github.com/briansmith/ring/pull/1554
# https://github.com/rust-lang/rustup/issues/2612#issuecomment-1433876793
# https://github.com/rustls/rustls/pull/1108
echo ring = { git = \"https://github.com/awakecoding/ring\", branch = \"0.16.20_alpha\" } >> ../cargo.toml
cd .. && cargo update 
cd pact_ffi

cargo build --target aarch64-pc-windows-msvc --release
gzip -c ../target/aarch64-pc-windows-msvc/release/pact_ffi.dll > ../target/artifacts/pact_ffi-windows-aarch64.dll.gz
openssl dgst -sha256 -r ../target/artifacts/pact_ffi-windows-aarch64.dll.gz > ../target/artifacts/pact_ffi-windows-aarch64.dll.gz.sha256
gzip -c ../target/aarch64-pc-windows-msvc/release/pact_ffi.dll.lib > ../target/artifacts/pact_ffi-windows-aarch64.dll.lib.gz
openssl dgst -sha256 -r ../target/artifacts/pact_ffi-windows-aarch64.dll.lib.gz > ../target/artifacts/pact_ffi-windows-aarch64.dll.lib.gz.sha256
gzip -c ../target/aarch64-pc-windows-msvc/release/pact_ffi.lib > ../target/artifacts/pact_ffi-windows-aarch64.lib.gz
openssl dgst -sha256 -r ../target/artifacts/pact_ffi-windows-aarch64.lib.gz > ../target/artifacts/pact_ffi-windows-aarch64.lib.gz.sha256