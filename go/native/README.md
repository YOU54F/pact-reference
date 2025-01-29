# Go Interop with Rust Issue

This is an example which shows a Rust project which has been compiled as a shared library, and is called from multiple languages including Go.

The rust project, may call out to external executables (called plugins) which can be written in any language.

This is currently causing issues with occasional segfaults in Linux / MacOS.

## Rust shared library

The rust shared library is currently built for the following platforms.

- x86_64-unknown-linux-gnu
- aarch64-unknown-linux-gnu
- x86_64-unknown-linux-musl
- aarch64-unknown-linux-musl
- x86_64-apple-darwin
- aarch64-apple-darwin
- x86_64-pc-windows-msvc
- aarch64-pc-windows-msvc

## Calling languages

It is called from various languages, including but not limited to:

- Golang (via cgo)
- JS (via node-napi)
- PHP (via FFI)
- Python (via cffi)

## Plugins, controlled by the rust shared library

The shared library, may load plugins which are written in any language, and may call out to external executables which are written in any language.

The following languages are currently being used as plugins:

- Golang
  - Built with GoReleaser for the following platforms:
    - linux/amd64
    - linux/arm64
    - darwin/amd64
    - darwin/arm64
    - windows/amd64
    - windows/arm64
- Rust
  - Built with cross / cargo
  - For linux, musl static builds are used to work on glibc & musl systems
    - x86_64-unknown-linux-musl
    - aarch64-unknown-linux-musl
    - x86_64-apple-darwin
    - aarch64-apple-darwin
    - x86_64-pc-windows-msvc
    - aarch64-pc-windows-msvc
- Java
  - Requires user to have openjdk 17 installed

## Issues

### Issue 1 - Segfaults on Linux Musl 

1. Test with CGO_ENABLED=0 using https://github.com/ebitengine/purego

## Testing

- Use of CGO or ebitengine/purego is controlled by `CGO_ENABLED=1` for `cgo` and `CGO_ENABLED=0` for `purego`
