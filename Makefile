PROTO=xcb-proto-1.15.2
PROTO_OUT=x11rb-protocol/src/protocol
X11RB_OUT=x11rb/src/protocol

generate:
	mkdir -p "$(PROTO_OUT)" "$(X11RB_OUT)"
	cargo run -p x11rb-generator -- "$(PROTO)/src" "$(PROTO_OUT)" "$(X11RB_OUT)"

.PHONY: generate
