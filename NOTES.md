cgo issue on alpine, sporadic segfaults


See following Dockerfiles for repro

- Dockerfile.alpine_cgo_issue_repro
- Dockerfile.alpine_pact_cgo_issue_repro

Example code

- `./go`
- https://github.com/pactflow/example-provider-golang

Test's different combos, see https://github.com/pactflow/example-provider-golang/pull/16 for more detail

- go versions
  - 1.22
  - 1.23
- cgo
  - on
  - off
- variant
  - alpine
  - debian
- implementation
  - pact-go
  - cgo when `CGO_ENABLED=1`
  - ebitengine/purego when `CGO_ENABLED=0`


``` sh
Starting program: /home/example-provider-golang/pact_go 
[New LWP 93]
[New LWP 94]
[New LWP 95]
[New LWP 96]
[New LWP 97]
[New LWP 98]
[New LWP 99]
[GIN-debug] [WARNING] Creating an Engine instance with the Logger and Recovery middleware already attached.

[GIN-debug] [WARNING] Running in "debug" mode. Switch to "release" mode in production.
 - using env:   export GIN_MODE=release
 - using code:  gin.SetMode(gin.ReleaseMode)

[GIN-debug] GET    /product/:id              --> github.com/pactflow/example-provider-golang.GetProduct (3 handlers)
[GIN-debug] [WARNING] You trusted all proxies, this is NOT safe. We recommend you to set a value.
Please check https://pkg.go.dev/github.com/gin-gonic/gin#readme-don-t-trust-all-proxies for details.
[GIN-debug] Listening and serving HTTP on :38927
[New LWP 100]
[New LWP 101]
[New LWP 102]
[New LWP 103]
[New LWP 104]
[New LWP 105]
[New LWP 106]
[New LWP 107]
[New LWP 108]
[New LWP 109]

Thread 4 "pact_go" received signal SIGSEGV, Segmentation fault.
[Switching to LWP 95]
0x0000fffff5935e84 in pact_ffi::verifier::handle::{impl#0}::execute::{async_block#1} () at pact_ffi/src/verifier/handle.rs:289
warning: 289    pact_ffi/src/verifier/handle.rs: No such file or directory
(gdb) bt
#0  0x0000fffff5935e84 in pact_ffi::verifier::handle::{impl#0}::execute::{async_block#1} () at pact_ffi/src/verifier/handle.rs:289
#1  0x0000fffff5a4b2b4 in tokio::task::task_local::{impl#4}::poll::{closure#0}<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}> () at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/task/task_local.rs:391
#2  0x0000fffff5a4bd90 in tokio::task::task_local::LocalKey<alloc::string::String>::scope_inner<alloc::string::String, tokio::task::task_local::{impl#4}::poll::{closure_env#0}<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, core::option::Option<core::task::poll::Poll<core::result::Result<pact_verifier::verification_result::VerificationExecutionResult, anyhow::Error>>>> (self=0xfffff7e20820 <pact_matching::logging::LOG_ID>, slot=0xffffad2a9dd8, f=...) at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/task/task_local.rs:217
#3  0x0000fffff5a4ae50 in tokio::task::task_local::{impl#4}::poll<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}> (self=..., cx=0xffffaea67d10) at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/task/task_local.rs:387
#4  0x0000fffff58b0060 in core::future::future::{impl#1}::poll<alloc::boxed::Box<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, alloc::alloc::Global>> (self=..., cx=0xffffaea67d10)
    at /rustc/90b35a6239c3d8bdabc530a6a0816f7ff89a0aaf/library/core/src/future/future.rs:123
#5  0x0000fffff59aadb8 in tokio::runtime::park::{impl#4}::block_on::{closure#0}<core::pin::Pin<alloc::boxed::Box<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, alloc::alloc::Global>>> ()
    at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/park.rs:284
#6  0x0000fffff59a770c in tokio::runtime::coop::with_budget<core::task::poll::Poll<core::result::Result<pact_verifier::verification_result::VerificationExecutionResult, anyhow::Error>>, tokio::runtime::park::{impl#4}::block_on::{closure_env#0}<core::pin::Pin<alloc::boxed::Box<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, alloc::alloc::Global>>>> (budget=..., f=...) at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/coop.rs:107
#7  tokio::runtime::coop::budget<core::task::poll::Poll<core::result::Result<pact_verifier::verification_result::VerificationExecutionResult, anyhow::Error>>, tokio::runtime::park::{impl#4}::block_on::{closure_env#0}<core::pin::Pin<alloc::boxed::Box<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, alloc::alloc::Global>>>> (f=...) at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/coop.rs:73
#8  tokio::runtime::park::CachedParkThread::block_on<core::pin::Pin<alloc::boxed::Box<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, alloc::alloc::Global>>> (self=0xffffaea67efe, f=...)
    at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/park.rs:284
#9  0x0000fffff5c70f74 in tokio::runtime::context::blocking::BlockingRegionGuard::block_on<core::pin::Pin<alloc::boxed::Box<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, alloc::alloc::Global>>> (self=0xffffaea68060, f=...)
    at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/context/blocking.rs:66
#10 0x0000fffff581acc0 in tokio::runtime::scheduler::multi_thread::{impl#0}::block_on::{closure#0}<core::pin::Pin<alloc::boxed::Box<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, alloc::alloc::Global>>> (blocking=0xffffaea68060)
    at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/scheduler/multi_thread/mod.rs:87
#11 0x0000fffff5c0b48c in tokio::runtime::context::runtime::enter_runtime<tokio::runtime::scheduler::multi_thread::{impl#0}::block_on::{closure_env#0}<core::pin::Pin<alloc::boxed::Box<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, alloc::alloc::Global>>>, core::result::Result<pact_verifier::verification_result::VerificationExecutionResult, anyhow::Error>> (handle=0xfffff7f3bae0 <<pact_ffi::RUNTIME as core::ops::deref::Deref>::deref::__stability::LAZY+48>, allow_block_in_place=true, f=...)
    at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/context/runtime.rs:65
#12 0x0000fffff581a954 in tokio::runtime::scheduler::multi_thread::MultiThread::block_on<core::pin::Pin<alloc::boxed::Box<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, alloc::alloc::Global>>> (
    self=0xfffff7f3bab8 <<pact_ffi::RUNTIME as core::ops::deref::Deref>::deref::__stability::LAZY+8>, handle=0xfffff7f3bae0 <<pact_ffi::RUNTIME as core::ops::deref::Deref>::deref::__stability::LAZY+48>, future=...)
    at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/scheduler/multi_thread/mod.rs:86
#13 0x0000fffff5a25a3c in tokio::runtime::runtime::Runtime::block_on_inner<core::pin::Pin<alloc::boxed::Box<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>, alloc::alloc::Global>>> (
    self=0xfffff7f3bab0 <<pact_ffi::RUNTIME as core::ops::deref::Deref>::deref::__stability::LAZY>, future=...) at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/runtime.rs:370
#14 0x0000fffff5a26738 in tokio::runtime::runtime::Runtime::block_on<tokio::task::task_local::TaskLocalFuture<alloc::string::String, pact_ffi::verifier::handle::{impl#0}::execute::{async_block_env#1}>> (self=0xfffff7f3bab0 <<pact_ffi::RUNTIME as core::ops::deref::Deref>::deref::__stability::LAZY>, future=...)
--Type <RET> for more, q to quit, c to continue without paging--
    at /root/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.43.0/src/runtime/runtime.rs:340
#15 0x0000fffff5c4cbcc in pact_ffi::verifier::handle::VerifierHandle::execute (self=0xfffff55d0660) at pact_ffi/src/verifier/handle.rs:289
#16 0x0000fffff5cfd280 in pact_ffi::verifier::pactffi_verifier_execute::{closure#0} () at pact_ffi/src/verifier/mod.rs:645
#17 0x0000fffff5c51320 in std::panicking::try::do_call<pact_ffi::verifier::pactffi_verifier_execute::{closure_env#0}, core::result::Result<i32, anyhow::Error>> (data=0xffffaea781f0) at /rustc/90b35a6239c3d8bdabc530a6a0816f7ff89a0aaf/library/std/src/panicking.rs:557
#18 0x0000fffff5a158fc in __rust_try () from /tmp/libpact_ffi.so
#19 0x0000fffff5a0fab4 in std::panicking::try<core::result::Result<i32, anyhow::Error>, pact_ffi::verifier::pactffi_verifier_execute::{closure_env#0}> (f=...) at /rustc/90b35a6239c3d8bdabc530a6a0816f7ff89a0aaf/library/std/src/panicking.rs:520
#20 std::panic::catch_unwind<pact_ffi::verifier::pactffi_verifier_execute::{closure_env#0}, core::result::Result<i32, anyhow::Error>> (f=...) at /rustc/90b35a6239c3d8bdabc530a6a0816f7ff89a0aaf/library/std/src/panic.rs:358
#21 0x0000fffff5c9adac in pact_ffi::error::panic::catch_panic<i32, pact_ffi::verifier::pactffi_verifier_execute::{closure_env#0}> (f=...) at pact_ffi/src/error/panic.rs:20
#22 0x0000fffff5b2b174 in pact_ffi::verifier::pactffi_verifier_execute (handle=0xfffff55d0660) at pact_ffi/src/util/ffi.rs:23
#23 0x0000000000acefc4 in _cgo_b3f3fec7e6e1_Cfunc_pactffi_verifier_execute (v=0x4000017578) at /tmp/go-build/cgo-gcc-prolog:141
#24 0x00000000004805ac in runtime.asmcgocall () at /usr/lib/go/src/runtime/asm_arm64.s:1000
#25 0x00000040001d16c0 in ?? ()
Backtrace stopped: not enough registers or memory available to unwind further
(gdb) 
(gdb) info frame
Stack level 0, frame at 0xffffaea5ffa0:
 pc = 0xfffff5935e84 in pact_ffi::verifier::handle::{impl#0}::execute::{async_block#1} (pact_ffi/src/verifier/handle.rs:289); saved pc = 0xfffff5a4b2b4
 called by frame at 0xffffaea67910
 source language rust.
 Arglist at 0xffffaea58f90, args: 
 Locals at 0xffffaea58f90, Previous frame's sp is 0xffffaea5ffa0
 Saved registers:
  x29 at 0xffffaea5ff90, x30 at 0xffffaea5ff98
```