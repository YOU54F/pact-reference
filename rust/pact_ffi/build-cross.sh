#!/bin/bash

set -e

echo -Setup directories --
# cargo clean
mkdir -p ../target/artifacts

echo -Build the Docker build image --
cargo install cross --git https://github.com/cross-rs/cross
targets=(
  x86_64-unknown-linux-gnu
  x86_64-unknown-linux-musl

  aarch64-unknown-linux-gnu
  aarch64-unknown-linux-musl

  arm-unknown-linux-gnueabi
  arm-unknown-linux-gnueabihf
  arm-unknown-linux-musleabi
  arm-unknown-linux-musleabihf

  armv7-unknown-linux-gnueabi
  armv7-unknown-linux-gnueabihf
  armv7-unknown-linux-musleabi
  armv7-unknown-linux-musleabihf

  i586-unknown-linux-gnu
  i686-unknown-linux-gnu

  i586-unknown-linux-musl
  i686-unknown-linux-musl

  x86_64-unknown-netbsd

  x86_64-unknown-freebsd
  i686-unknown-freebsd
  aarch64-unknown-freebsd
  armv6-unknown-freebsd
  armv7-unknown-freebsd

  armv5te-unknown-linux-gnueabi
  armv5te-unknown-linux-musleabi
  arm-linux-androideabi
  armv7-linux-androideabi

  mips-unknown-linux-gnu
  mips-unknown-linux-musl
  mipsel-unknown-linux-gnu
  mipsel-unknown-linux-musl
  mips64-unknown-linux-gnuabi64
  mips64el-unknown-linux-gnuabi64
  mips64-unknown-linux-muslabi64
  mips64el-unknown-linux-muslabi64

  powerpc-unknown-linux-gnu
  powerpc64-unknown-linux-gnu
  powerpc64le-unknown-linux-gnu
  powerpc64le-unknown-linux-musl

  riscv64gc-unknown-linux-gnu
  s390x-unknown-linux-gnu
  s390x-unknown-linux-musl
  sparc64-unknown-linux-gnu

  aarch64-linux-android
  i686-linux-android
  x86_64-linux-android

  wasm32-unknown-emscripten
  i686-unknown-freebsd
  sparcv9-sun-solaris

  x86_64-sun-solaris
  x86_64-unknown-illumos
  x86_64-unknown-dragonfly

  thumbv6m-none-eabi
  thumbv7em-none-eabi
  thumbv7em-none-eabihf
  thumbv7m-none-eabi
  thumbv7neon-linux-androideabi
  thumbv7neon-unknown-linux-gnueabihf
  thumbv8m.base-none-eabi
  thumbv8m.main-none-eabi

  x86_64-pc-windows-msvc # pass on win
  x86_64-pc-windows-gnu # pass on win / linux cross

  aarch64-pc-windows-msvc  # pass on win
  aarch64-pc-windows-gnullvm
  aarch64-uwp-windows-msvc

  i586-pc-windows-msvc
  i686-pc-windows-gnu  # pass on win
  i686-pc-windows-msvc
  i686-uwp-windows-gnu 
  i686-uwp-windows-msvc

  thumbv7a-pc-windows-msvc
  thumbv7a-uwp-windows-msvc

  x86_64-pc-windows-gnullvm
  x86_64-uwp-windows-gnu 
  x86_64-uwp-windows-msvc

)

if [[ ! -n $target ]]; then
  echo "building for all targets: ${targets[@]}"
else
  targets=($target)
fi

for target in "${targets[@]}"; do

  if [[ $target == "aarch64-pc-windows-msvc" ]]; then
    ## The following is a workaround until ring 16 supports windows arm64 or rustls moves to ring 17 (post release)
    ## It also relies on ../cargo.toml having the [patch.crates-io] section at the bottom of the file
    # https://github.com/briansmith/ring/issues/1514#issuecomment-1258562375
    # https://github.com/briansmith/ring/pull/1554
    # https://github.com/rust-lang/rustup/issues/2612#issuecomment-1433876793
    # https://github.com/rustls/rustls/pull/1108
    echo ring = { git = \"https://github.com/awakecoding/ring\", branch = \"0.16.20_alpha\" } >>cargo.toml
    cargo update
  fi

  if [[ $target == *"musl"* ]]; then
    # https://github.com/rust-lang/cargo/issues/7154
    RUSTFLAGS="-C target-feature=-crt-static" cross build --target "${target}" --release
  else
    cross build --target "${target}" --release
  fi

  if [[ $target == *"windows"* ]]; then
    lib_ext=dll
    lib_name=pact_ffi
  elif [[ $target == *"apple"* ]]; then
    lib_ext=dylib
    lib_name=libpact_ffi
  else
    lib_ext=so
    lib_name=libpact_ffi
  fi


  ls ../target/${target}/release
  echo -Build the release artifacts --
  ## cdylib - shared lib .so / .dll / .dylib depending on platform  
  gzip -c ../target/${target}/release/${lib_name}.${lib_ext} >../target/artifacts/${lib_name}-${target}.${lib_ext}.gz
  openssl dgst -sha256 -r ../target/artifacts/${lib_name}-${target}.${lib_ext}.gz >../target/artifacts/${lib_name}-${target}.${lib_ext}.sha256

  if [[ $target == *"windows"* ]]; then
    ## dll.lib
    lib_ext=dll.lib
    gzip -c ../target/${target}/release/${lib_name}.${lib_ext} >../target/artifacts/${lib_name}-${target}.${lib_ext}.gz
    openssl dgst -sha256 -r ../target/artifacts/${lib_name}-${target}.${lib_ext}.gz >../target/artifacts/${lib_name}-${target}.${lib_ext}.sha256
    ## lib
    lib_ext=lib
    gzip -c ../target/${target}/release/${lib_name}.${lib_ext} >../target/artifacts/${lib_name}-${target}.${lib_ext}.gz
    openssl dgst -sha256 -r ../target/artifacts/${lib_name}-${target}.${lib_ext}.gz >../target/artifacts/${lib_name}-${target}.${lib_ext}.sha256
  elif [[ $target == *"ios"* ]]; then
      ## static lib .a 
      ## these are 30mb each, I'm not sure who needs them?
      ## ios platforms maybe, there is also refs in conan
      ## but they are build with cargo lipo, see release-ios.sh
      gzip -c ../target/${target}/release/${lib_name}.a >../target/artifacts/${lib_name}-${target}.a.gz
      openssl dgst -sha256 -r ../target/artifacts/${lib_name}-${target}.a.gz >../target/artifacts/${lib_name}-${target}.a.gz.sha256
  fi

done

ls ../target/artifacts
