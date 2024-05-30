package main

// #cgo CFLAGS: -g -I../../rust/pact_ffi/include/ -I../../rust/release_artifacts
// #cgo linux,arm64 LDFLAGS: -lpact_ffi -L../../rust/target/aarch64-unknown-linux-musl/release/
// #cgo linux,amd64 LDFLAGS: -lpact_ffi -L../../rust/target/x86_64-unknown-linux-musl/release/
// #cgo darwin,amd64 LDFLAGS: -lpact_ffi -L${SRCDIR}/../../rust/target/x86_64-apple-darwin/release/
// #cgo darwin,arm64 LDFLAGS: -lpact_ffi -L${SRCDIR}/../../rust/target/x86_64-apple-darwin/release/
// #cgo windows,amd64 LDFLAGS: -lpact_ffi -L${SRCDIR}/../../rust/target/aarch64-pc-windows-msvc/release/
// #cgo windows,arm64 LDFLAGS: -lpact_ffi -L${SRCDIR}/../../rust/target/x86_64-pc-windows-msvc/release/
// #include "pact.h"
import "C"
import (
	"fmt"
)

func main() {
	version := C.pactffi_version()
	// fmt.Println(C.GoString(version))
	C.pactffi_logger_init()
	C.pactffi_logger_attach_sink(C.CString("stdout"), 3)
	C.pactffi_logger_apply()
	C.pactffi_log_message(C.CString("pact-go-ffi"), C.CString("INFO"), C.CString(fmt.Sprintf("hello from ffi version: %s", C.GoString(version))))
}
