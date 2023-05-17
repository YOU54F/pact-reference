#!/bin/bash

set -e
set -x

rustc --version

cargo install --force cbindgen
rm -rf ./include
rm -rf ../target/artifacts
mkdir -p ../target/artifacts

rustup toolchain install nightly

echo -------------------------------------
echo - Build library with CMake
echo -------------------------------------
mkdir -p build
cd build
cmake -DCMAKE_BUILD_TYPE=Debug ..
cmake --build . -v
cd ..

echo -------------------------------------
echo - Generate header with cbindgen
echo -------------------------------------
rustup run nightly cbindgen \
  --config cbindgen.toml \
  --crate pact_ffi \
  --output include/pact.h
rustup run nightly cbindgen \
  --config cbindgen-c++.toml \
  --crate pact_ffi \
  --output include/pact-c++.h

echo -------------------------------------
echo - Copy headers to artifacts for release
echo -------------------------------------

cp include/*.h ../target/artifacts
ls ../target/artifacts

echo -------------------------------------
echo - Make library available for examples
echo -------------------------------------
cd build
cmake --install . --prefix ./install

echo -------------------------------------
echo - Running examples
echo -------------------------------------
cd ..
for i in examples/*; do
  pushd $i
  mkdir -p build
  cd build
  cmake ..
  cmake --build .

  echo "Running example $i"
  if [[ "$OSTYPE" == "msys"* ]]; then
    cp ../../../build/install/lib/*.dll Debug/
    ./Debug/example.exe
  elif [[ "$OSTYPE" == "darwin"* ]]; then
    cp ../../../build/install/lib/*.dylib .
    ./example
  else
    ./example
  fi

  popd
done
