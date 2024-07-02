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

### Issue 1 - Segfaults on Linux / MacOS

```
signal 17 received but handler not on signal stack
mp.gsignal stack [0x400006e000 0x4000076000], mp.g0 stack [0xffff4320e810 0xffff43a0e410], sp=0x40000e3608
fatal error: non-Go code set up signal handler without SA_ONSTACK flag
```

Fixed by workaround in following PR

- https://github.com/wailsapp/wails/pull/2152/files#diff-d4a0fa73df7b0ab971e550f95249e358b634836e925ace96f7400480916ac09e

> Sets up a new signal handler in C that overrides the current one (in C) so that SA_ONSTACK is used.

See golang docs for os/signal 

https://pkg.go.dev/os/signal#hdr-Go_programs_that_use_cgo_or_SWIG

> If the non-Go code installs any signal handlers, it must use the SA_ONSTACK flag with sigaction. Failing to do so is likely to cause the program to crash if the signal is received. Go programs routinely run with a limited stack, and therefore set up an alternate signal stack.

#### Additional Problems

1. Does not fix Alpine linux
2. Fix does not with CGO_ENABLED=0 using https://github.com/ebitengine/purego, as it requires C code to be injected.
   1. Probably fix needs to be applied to fakego in purego

## Testing

- Use of CGO or ebitengine/purego is controlled by `CGO_ENABLED=1` for `cgo` and `CGO_ENABLED=0` for `purego`
- Fix can be removed by settings `SKIP_SIGNAL_HANDLERS=true`

### Issue 2 - Windows plugin executables are not shutdown properly

```
*** Test I/O incomplete 1m0s after exiting.
exec: WaitDelay expired before I/O complete
```

The windows executable is shutdown externally via taskkill or the task manager, will exit the plugin correctly and the test will pass.
