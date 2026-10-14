CARGO        ?= cargo
CARGO_FLAGS  ?=
BIN          := termix
TARGET_DIR   := target
FEATURES     ?=

# Đảm bảo cargo/rustc có trong PATH ngay cả khi make chạy trong non-login shell.
# .cargo/env và .local/bin đã có trong ~/.bashrc nhưng non-interactive make không load nó.
export PATH := $(HOME)/.cargo/bin:$(HOME)/.local/bin:$(HOME)/.bun/bin:/usr/local/go/bin:$(PATH)

# ── colours (disabled when not a TTY) ────────────────────────────────────────
ifeq ($(shell test -t 1 && echo yes),yes)
  B  := \033[1m
  D  := \033[2m
  C  := \033[36m
  G  := \033[32m
  Y  := \033[33m
  R  := \033[0m
else
  B := \033[1m
  D := \033[2m
  C := \033[36m
  G := \033[32m
  Y := \033[33m
  R := \033[0m
endif

.DEFAULT_GOAL := help
SHELL         := /usr/bin/env bash
.SHELLFLAGS   := -eu -o pipefail -c

# ── helpers ──────────────────────────────────────────────────────────────────
define log
	@printf "$(C)▸$(R) $(B)%s$(R)\n" "$(1)"
endef

# =============================================================================
#  help — list every target with its doc comment (##)
# =============================================================================
.PHONY: help
help: ## Hiện danh sách các lệnh khả dụng
	@printf "\n$(B)Termix$(R) $(D)— v0.2.112$(R)\n\n"
	@printf "$(B)Usage:$(R)  make $(Y)<target>$(R)\n\n"
	@awk 'BEGIN { FS = ":.*##" } /^[a-zA-Z0-9_\/-]+:.*##/ { printf "  $(C)%-22s$(R) %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@printf "\n$(D)Biến môi trường:$(R)\n"
	@printf "  $(Y)CARGO$(R)        Trình cargo (mặc định: cargo)\n"
	@printf "  $(Y)CARGO_FLAGS$(R)  Flags thêm cho cargo\n"
	@printf "  $(Y)FEATURES$(R)     Cargo features (ví dụ: make build FEATURES=browser-fixture)\n"
	@printf "\n"

# =============================================================================
#  deps — cài thư viện hệ thống (Linux / Ubuntu 24.04+)
# =============================================================================
.PHONY: deps deps-check
deps: ## Cài toàn bộ thư viện hệ thống cần thiết để build (cần sudo)
	$(call log,Cài đặt system dependencies …)
	sudo apt update
	sudo apt install -y \
		pkg-config \
		libssl-dev \
		libasound2-dev \
		libfontconfig1-dev libfreetype6-dev libexpat1-dev \
		libxkbcommon-dev libxkbcommon-x11-dev \
		libwayland-dev libxcb1-dev libxcb-xkb-dev \
		libvulkan-dev \
		libwebkit2gtk-4.1-dev libjson-glib-dev \
		clang mold
	@printf "$(G)✔ Dependencies đã sẵn sàng.$(R)\n"

deps-check: ## Kiểm tra xem các thư viện hệ thống đã đủ chưa
	@missing=""; \
	for pc in openssl fontconfig webkit2gtk-4.1 json-glib-1.0 xkbcommon xkbcommon-x11 wayland-client; do \
		if ! pkg-config --exists "$$pc" 2>/dev/null; then \
			missing="$$missing $$pc"; \
		fi; \
	done; \
	if [ -n "$$missing" ]; then \
		printf "$(Y)⚠ Thiếu pkg-config packages:$(R)$$missing\n"; \
		printf "  Chạy: $(B)make deps$(R)\n"; \
		exit 1; \
	fi; \
	printf "$(G)✔ Tất cả system dependencies đã đủ.$(R)\n"

# =============================================================================
#  build
# =============================================================================
.PHONY: build release
build: ## Build debug (cargo build)
	$(call log,Building debug …)
	$(CARGO) build $(CARGO_FLAGS) $(if $(FEATURES),--features $(FEATURES),)

release: ## Build release (thin LTO, stripped)
	$(call log,Building release …)
	$(CARGO) build --release $(CARGO_FLAGS) $(if $(FEATURES),--features $(FEATURES),)

# =============================================================================
#  check / clippy / fmt
# =============================================================================
.PHONY: check clippy fmt fmt-check
check: ## cargo check toàn workspace
	$(CARGO) check --workspace $(CARGO_FLAGS) $(if $(FEATURES),--features $(FEATURES),)

clippy: ## Chạy clippy với -D warnings
	$(CARGO) clippy --workspace --all-targets -- -D warnings

fmt: ## Format toàn bộ code Rust (cargo fmt)
	$(CARGO) fmt --all

fmt-check: ## Kiểm tra format (không ghi file, dùng cho CI)
	$(CARGO) fmt --all -- --check

# =============================================================================
#  test
# =============================================================================
.PHONY: test test-verbose test-one

test: ## Chạy toàn bộ tests
	$(CARGO) test --workspace $(CARGO_FLAGS) $(if $(FEATURES),--features $(FEATURES),)

test-verbose: ## Chạy tests với output chi tiết
	$(CARGO) test --workspace -- --nocapture

test-one: ## Chạy 1 test cụ thể (dùng: make test-one TEST=foo)
	@if [ -z "$(TEST)" ]; then echo "Dùng: make test-one TEST=<tên_test>"; exit 1; fi
	$(CARGO) test --workspace -- $(TEST) --nocapture

# =============================================================================
#  lint — gộp mọi kiểm tra (dùng cho CI / pre-push)
# =============================================================================
.PHONY: lint tidy
lint: fmt-check clippy ## Chạy fmt-check + clippy (alias của tidy)
tidy: fmt-check clippy check ## Chạy fmt-check + clippy + check

# =============================================================================
#  run
# =============================================================================
.PHONY: run
run: ## Chạy binary termix (chuyển args qua ARGS, ví dụ: make run ARGS="--help")
	$(CARGO) run --bin $(BIN) $(CARGO_FLAGS) -- $(ARGS)

# =============================================================================
#  clean
# =============================================================================
.PHONY: clean clean-all
clean: ## Xóa artifacts build (cargo clean)
	$(CARGO) clean

clean-all: clean ## Xóa cả target/ và Cargo.lock cache
	rm -rf $(TARGET_DIR)

# =============================================================================
#  audit / outdated (cần cargo-audit / cargo-outdated)
# =============================================================================
.PHONY: audit outdated update
audit: ## Quét lỗ hổng bảo mật (cần cargo install cargo-audit)
	$(CARGO) audit

outdated: ## Liệt kê dependencies lỗi thời (cần cargo install cargo-outdated)
	$(CARGO) outdated --workspace

update: ## Cập nhật Cargo.lock (cargo update)
	$(CARGO) update

# =============================================================================
#  install — cài binary vào ~/.cargo/bin hoặc PREFIX
# =============================================================================
PREFIX ?= $(HOME)/.cargo

.PHONY: install install-release
install: ## Cài binary debug vào $$(PREFIX)/bin
	$(CARGO) install --path app --force --root $(PREFIX)

install-release: ## Cài binary release vào $$(PREFIX)/bin
	$(CARGO) install --path app --force --root $(PREFIX) --profile release

# =============================================================================
#  doctor — chẩn đoán môi trường build
# =============================================================================
.PHONY: doctor
doctor: ## In thông tin môi trường build để debug
	@printf "$(B)Toolchain$(R)\n"
	@printf "  cargo    : $$($(CARGO) --version 2>&1)\n"
	@printf "  rustc    : $$(rustc --version 2>&1)\n"
	@printf "  rustfmt  : $$(rustfmt --version 2>&1)\n"
	@printf "  go       : $$(go version 2>&1)\n"
	@printf "  node     : $$(node --version 2>&1 || echo 'not found')\n"
	@printf "  bun      : $$(bun --version 2>&1 || echo 'not found')\n"
	@printf "  clang    : $$(clang --version 2>&1 | head -1)\n"
	@printf "  mold     : $$(mold --version 2>&1 | head -1 || ld.mold --version 2>&1 | head -1)\n"
	@printf "  pkg-config: $$(pkg-config --version 2>&1)\n"
	@printf "\n$(B)System libs (pkg-config)$(R)\n"
	@for pc in openssl fontconfig webkit2gtk-4.1 json-glib-1.0 xkbcommon xkbcommon-x11 wayland-client xcb vulkan; do \
		if pkg-config --exists "$$pc" 2>/dev/null; then \
			printf "  $(G)✔$(R) %-22s %s\n" "$$pc" "$$(pkg-config --modversion "$$pc" 2>/dev/null)"; \
		else \
			printf "  $(Y)✘$(R) %-22s (not found)\n" "$$pc"; \
		fi; \
	done
	@printf "\n$(B)Cargo workspace$(R)\n"
	@printf "  members  : $$(grep -c '\"crates/' Cargo.toml) crates + app\n"
	@printf "  version  : $$(grep '^version' Cargo.toml | head -1)\n"
