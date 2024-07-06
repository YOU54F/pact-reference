//go:build cgo
// +build cgo

// Package native contains the c bindings into the Pact Reference types.
package main

/*
#cgo darwin,arm64 LDFLAGS: -L/tmp -L/usr/local/lib -Wl,-rpath -Wl,/tmp -Wl,-rpath -Wl,/usr/local/lib -lpact_ffi
#cgo darwin,amd64 LDFLAGS: -L/tmp -L/usr/local/lib -Wl,-rpath -Wl,/tmp -Wl,-rpath -Wl,/usr/local/lib -lpact_ffi
#cgo windows,amd64 LDFLAGS: -lpact_ffi
#cgo linux,amd64 LDFLAGS: -L/tmp -L/opt/pact/lib -L/usr/local/lib -Wl,-rpath -Wl,/opt/pact/lib -Wl,-rpath -Wl,/tmp -Wl,-rpath -Wl,/usr/local/lib -lpact_ffi
#cgo linux,arm64 LDFLAGS: -L/tmp -L/opt/pact/lib -L/usr/local/lib -Wl,-rpath -Wl,/opt/pact/lib -Wl,-rpath -Wl,/tmp -Wl,-rpath -Wl,/usr/local/lib -lpact_ffi
#include "pact.h"
*/
import "C"
import (
	"fmt"
	"os"
)

func plugin_provider() int {
	verifier := C.pactffi_verifier_new()
	C.pactffi_log_to_stdout(4)
	C.pactffi_verifier_set_provider_info(verifier, C.CString("p1"), C.CString("http"), C.CString("localhost"), 8000, C.CString("/"))
	// C.pactffi_verifier_add_provider_transport(verifier, C.CString("http"), 8000, C.CString("/"), C.CString("http"))
	// C.pactffi_verifier_add_provider_transport(verifier, C.CString("protobuf"), 37757, C.CString("/"), C.CString("tcp"))

	C.pactffi_verifier_add_directory_source(verifier, C.CString(os.Getenv("PACT_PROVIDER_DIR")))
	// InstallSignalHandlers()
	defer C.pactffi_verifier_shutdown(verifier)
	result := C.pactffi_verifier_execute(verifier)
	if result != 0 {
		fmt.Printf("Result is not 0: %d", result)
	} else {
		fmt.Print("Result success")
	}
	return int(result)
}
