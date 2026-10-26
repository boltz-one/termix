# Makefile — termix workspace (Windows-only Rust port of WT).
#
# Convention: target name = action; không prefix `@` (PowerShell/Git Bash tự echo).
# Workspace: 14 crates, binary `termix` từ `app`, ConPTY/Windows deps.
#
# Usage:
#   make              — check (default, no-op safe)
#   make build        — cargo build --workspace
#   make test         — toàn bộ unit/integration tests
#   make run          — chạy binary termix (cần gpui desktop)
#   make clippy       — lint toàn workspace
#   make check PKG=core  — check riêng 1 crate

PKG  ?=
CRATE := $(if $(PKG),-p $(PKG),--workspace)

# Phony: tất cả target đều action, không file output.
.PHONY: default check build run test clippy fmt fmt-check clean doc \
        build-release test-one quick ports

default: check

# — Build / check —

check:
	cargo check $(CRATE)

build:
	cargo build $(CRATE)

build-release:
	cargo build --release $(CRATE)

# Binary — `termix` (app). Yêu cầu desktop session + gpui backend.
run:
	cargo run -p app

# — Tests —

test:
	cargo test $(CRATE) --lib

# Test riêng: make test-one PKG=core T=sgr_stack
T ?=
test-one:
	cargo test -p $(PKG) --lib $(if $(T),$(T),)

# — Lint / format —

clippy:
	cargo clippy $(CRATE) -- -D warnings

fmt:
	cargo fmt $(if $(PKG),-p $(PKG),)

fmt-check:
	cargo fmt $(if $(PKG),-p $(PKG),) -- --check

# — Docs / clean —

doc:
	cargo doc $(CRATE) --no-deps

clean:
	cargo clean

# — Composite —

# Quick check cycle: fmt + check + test workspace.
quick: fmt-check check test

# — Port-specific (Probe WT source cho parity check) —
# Cần `windows/` repo sibling (port source-of-truth). Liệt kê file chưa port.
PORT_DIRS := $(abspath ../windows/src/terminal)/adapter:$(abspath ../windows/src/terminal)/renderer
ports:
	@echo "WT source dirs (parity baseline):"
	@echo "  $(PORT_DIRS)" | tr ':' '\n' | sed 's/^/    /'
