//go:build cgo
// +build cgo

// Package native contains the c bindings into the Pact Reference types.
package main

// extern void cgoTraceback(void*);
// extern void cgoSymbolizer(void*);
/*
#cgo darwin,arm64 LDFLAGS: -L/tmp -L/usr/local/lib -Wl,-rpath -Wl,/tmp -Wl,-rpath -Wl,/usr/local/lib -lpact_ffi
#cgo darwin,amd64 LDFLAGS: -L/tmp -L/usr/local/lib -Wl,-rpath -Wl,/tmp -Wl,-rpath -Wl,/usr/local/lib -lpact_ffi
#cgo windows,amd64 LDFLAGS: -lpact_ffi
#cgo linux,amd64 LDFLAGS: -L/tmp -L/home/pact-reference/rust/target/debug -L/usr/local/lib -Wl,-rpath -Wl,/home/pact-reference/rust/target/debug -Wl,-rpath -Wl,/tmp -Wl,-rpath -Wl,/usr/local/lib -lpact_ffi
#cgo linux,arm64 LDFLAGS: -L/tmp -L/home/pact-reference/rust/target/debug -L/usr/local/lib -Wl,-rpath -Wl,/home/pact-reference/rust/target/debug -Wl,-rpath -Wl,/tmp -Wl,-rpath -Wl,/usr/local/lib -lpact_ffi
#include "pact.h"
struct contextArg {
	uintptr_t context;
};

void cgoContext(struct contextArg* p) {
	if (p->context == 0) {
		p->context = 1;
	}
}

*/
import "C"
import (
	"fmt"
	"os"
	"runtime"

	"unsafe"

	_ "github.com/ianlancetaylor/cgosymbolizer"
)

func plugin_provider() int {
	runtime.SetCgoTraceback(0, unsafe.Pointer(C.cgoTraceback), unsafe.Pointer(C.cgoContext), unsafe.Pointer(C.cgoSymbolizer))
	verifier := C.pactffi_verifier_new()
	C.pactffi_verifier_set_provider_info(verifier, C.CString("p1"), C.CString("http"), C.CString("localhost"), 8000, C.CString("/"))

	C.pactffi_verifier_add_directory_source(verifier, C.CString(os.Getenv("PACT_PROVIDER_DIR")))
	defer C.pactffi_verifier_shutdown(verifier)
	result := C.pactffi_verifier_execute(verifier)
	if result != 0 {
		fmt.Printf("Result is not 0: %d", result)
	} else {
		fmt.Print("Result success")
	}
	return int(result)
}

type MessagePact struct {
	handle C.PactHandle
}

type messageType int

const (
	MESSAGE_TYPE_ASYNC messageType = iota
	MESSAGE_TYPE_SYNC
)

type Message struct {
	handle      C.InteractionHandle
	messageType messageType
	pact        *MessagePact
	index       int
	server      *MessageServer
}

// MessageServer is the public interface for managing the message based interface
type MessageServer struct {
	messagePact *MessagePact
	messages    []*Message
}

type interactionPart int

const (
	INTERACTION_PART_REQUEST interactionPart = iota
	INTERACTION_PART_RESPONSE
)

func free(str *C.char) {
	C.free(unsafe.Pointer(str))
}

func message_consumer_test() {
	C.pactffi_log_to_stdout(5)
	var consumer = "foo"
	var provider = "bar"
	var description = "a desc"
	var state = "test"
	var expects = "expects"
	cConsumer := C.CString(consumer)
	cProvider := C.CString(provider)
	defer free(cConsumer)
	defer free(cProvider)

	var m = &MessageServer{messagePact: &MessagePact{handle: C.pactffi_new_message_pact(cConsumer, cProvider)}}

	cDescription := C.CString(description)
	defer free(cDescription)

	interaction := &Message{
		handle:      C.pactffi_new_message_interaction(m.messagePact.handle, cDescription),
		messageType: MESSAGE_TYPE_ASYNC,
		pact:        m.messagePact,
		index:       len(m.messages),
		server:      m,
	}
	m.messages = append(m.messages, interaction)

	cState := C.CString(state)
	defer free(cState)

	C.pactffi_given(interaction.handle, cState)

	cExpects := C.CString(expects)
	defer free(cExpects)

	C.pactffi_message_expects_to_receive(interaction.handle, cExpects)

	var contentType = "text/plain"
	var part = INTERACTION_PART_REQUEST
	var body = []byte("some string")
	cHeader := C.CString(contentType)
	defer free(cHeader)

	res := C.pactffi_with_body(interaction.handle, uint32(part), cHeader, (*C.char)(unsafe.Pointer(&body[0])))
	print(bool(res))
	iter := C.pactffi_pact_handle_get_message_iter(m.messagePact.handle)
	if iter == nil {
		print("unable to get a message iterator")
	}
	print("[DEBUG] pactffi_pact_handle_get_message_iter - len", len(m.messages))
	for i := 0; i < len(m.messages); i++ {
		print("[DEBUG] pactffi_pact_handle_get_message_iter - index", i)
		message := C.pactffi_pact_message_iter_next(iter)
		print("[DEBUG] pactffi_pact_message_iter_next - message", message)

		if i == interaction.index {
			print("[DEBUG] pactffi_pact_message_iter_next - index match", message)

			if message == nil {
				print("retrieved a null message pointer")
				return
			}

			len := C.pactffi_message_get_contents_length(message)
			print("[DEBUG] pactffi_message_get_contents_length - len", len)
			if len == 0 {
				// You can have empty bodies
				print("[DEBUG] message body is empty")
				return
			}
			data := C.pactffi_message_get_contents_bin(message)
			print("[DEBUG] pactffi_message_get_contents_bin - data", data)
			if data == nil {
				// You can have empty bodies
				print("[DEBUG] message binary contents are empty")
				return
			}
			ptr := unsafe.Pointer(data)
			bytes := C.GoBytes(ptr, C.int(len))

			print("got bytes")
			print(string(bytes))
			return
		}
	}
}
