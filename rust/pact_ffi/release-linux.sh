#!/bin/bash

set -e
set -x

RUST_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")/.." && pwd )"

source "$RUST_DIR/scripts/gzip-and-sum.sh"
ARTIFACTS_DIR=${ARTIFACTS_DIR:-"$RUST_DIR/release_artifacts"}
mkdir -p "$ARTIFACTS_DIR"
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$RUST_DIR/target"}
# Create slim builds for release
export RUSTFLAGS="-C opt-level=z -C codegen-units=1 -C strip=symbols" 

# All flags passed to this script are passed to cargo.
cargo_flags=( "$@" )

install_cross() {
    cargo install cross@0.2.5 --force
}
install_cross_latest() {
    cargo install cross --git https://github.com/cross-rs/cross --force
}
# Remove the release/build folder between cross runs to avoid conflicting linker symbol errors
clean_cargo_release_build() {
    rm -rf $CARGO_TARGET_DIR/release/build
}

# Build the x86_64 GNU linux release
build_x86_64_gnu() {
    clean_cargo_release_build
    cross build --target x86_64-unknown-linux-gnu "${cargo_flags[@]}"

    if [[ "${cargo_flags[*]}" =~ "--release" ]]; then
        gzip_and_sum \
            "$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release/libpact_ffi.a" \
            "$ARTIFACTS_DIR/libpact_ffi-linux-x86_64.a.gz"
        gzip_and_sum \
            "$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release/libpact_ffi.so" \
            "$ARTIFACTS_DIR/libpact_ffi-linux-x86_64.so.gz"
    fi
}

build_aarch64_gnu() {
    clean_cargo_release_build
    cross build --target aarch64-unknown-linux-gnu "${cargo_flags[@]}"

    if [[ "${cargo_flags[*]}" =~ "--release" ]]; then
        gzip_and_sum \
            "$CARGO_TARGET_DIR/aarch64-unknown-linux-gnu/release/libpact_ffi.a" \
            "$ARTIFACTS_DIR/libpact_ffi-linux-aarch64.a.gz"
        gzip_and_sum \
            "$CARGO_TARGET_DIR/aarch64-unknown-linux-gnu/release/libpact_ffi.so" \
            "$ARTIFACTS_DIR/libpact_ffi-linux-aarch64.so.gz"
    fi
}

build_x86_64_musl() {
    clean_cargo_release_build
    # Set -crt-static, to build dynamic *.so library
    export RUSTFLAGS+="-C target-feature=-crt-static" 
    cross build --target x86_64-unknown-linux-musl "${cargo_flags[@]}"
    # Unset -crt-static, to ensure
    export RUSTFLAGS="${RUSTFLAGS//-C target-feature=-crt-static}"

    if [[ "${cargo_flags[*]}" =~ "--release" ]]; then
        gzip_and_sum \
            "$CARGO_TARGET_DIR/x86_64-unknown-linux-musl/release/libpact_ffi.a" \
            "$ARTIFACTS_DIR/libpact_ffi-linux-x86_64-musl.a.gz"
        gzip_and_sum \
            "$CARGO_TARGET_DIR/x86_64-unknown-linux-musl/release/libpact_ffi.so" \
            "$ARTIFACTS_DIR/libpact_ffi-linux-x86_64-musl.so.gz"
    fi
}

build_aarch64_musl() {
    clean_cargo_release_build
    export RUSTFLAGS+="-C target-feature=-crt-static" 
    cross build --target aarch64-unknown-linux-musl "${cargo_flags[@]}"
    export RUSTFLAGS="${RUSTFLAGS//-C target-feature=-crt-static}"

    if [[ "${cargo_flags[*]}" =~ "--release" ]]; then
        gzip_and_sum \
            "$CARGO_TARGET_DIR/aarch64-unknown-linux-musl/release/libpact_ffi.a" \
            "$ARTIFACTS_DIR/libpact_ffi-linux-aarch64-musl.a.gz"
        gzip_and_sum \
            "$CARGO_TARGET_DIR/aarch64-unknown-linux-musl/release/libpact_ffi.so" \
            "$ARTIFACTS_DIR/libpact_ffi-linux-aarch64-musl.so.gz"
    fi
}

build_header() {
    rustup toolchain install nightly
    rustup run nightly cbindgen \
        --config cbindgen.toml \
        --crate pact_ffi \
        --output "$ARTIFACTS_DIR/pact.h"
    rustup run nightly cbindgen \
        --config cbindgen-c++.toml \
        --crate pact_ffi \
        --output "$ARTIFACTS_DIR/pact-cpp.h"
}

install_cross
build_x86_64_gnu
build_aarch64_gnu
build_x86_64_musl
build_aarch64_musl
build_header