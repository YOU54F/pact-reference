#!/bin/bash -xe

cargo clean

mkdir -p ../target/artifacts

CRATE=pact_mock_server_cli
echo -- Build the x86_64 release artifacts --
TARGET=x86_64-unknown-linux-gnu
RELEASE_NAME=linux-x86_64
rustup target add ${TARGET}
cargo build --release --target=${TARGET}
gzip -c ../target/${TARGET}release/${CRATE} > ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz
openssl dgst -sha256 -r ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz > ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz.sha256

echo -- Build the aarch64 release artifacts --
TARGET=aarch64-unknown-linux-gnu
RELEASE_NAME=linux-aarch64
rustup target add ${TARGET}
cross build --release --target=${TARGET}
gzip -c ../target/${TARGET}release/${CRATE} > ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz
openssl dgst -sha256 -r ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz > ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz.sha256


echo -- Build the x86_64 musl release artifacts --
sudo apt install musl-tools
TARGET=x86_64-unknown-linux-musl
RELEASE_NAME=linux-musl-x86_64
rustup target add ${TARGET}
cargo build --release --target=${TARGET}
gzip -c ../target/${TARGET}release/${CRATE} > ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz
openssl dgst -sha256 -r ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz > ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz.sha256

echo -- Build the aarch64 musl release artifacts --
TARGET=aarch64-unknown-linux-musl
RELEASE_NAME=linux-musl-aarch64
rustup target add ${TARGET}
cross build --release --target=${TARGET}
gzip -c ../target/${TARGET}release/${CRATE} > ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz
openssl dgst -sha256 -r ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz > ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz.sha256

echo -- Build the armv7 release artifacts --
TARGET=armv7-unknown-linux-gnueabihf
RELEASE_NAME=linux-armv7-gnueabihf
rustup target add ${TARGET}
cross build --release --target=${TARGET}
gzip -c ../target/${TARGET}release/${CRATE} > ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz
openssl dgst -sha256 -r ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz > ../target/artifacts/${CRATE}-${RELEASE_NAME}.gz.sha256
