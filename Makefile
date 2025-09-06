# Parsec Workspace Makefile
# Usage examples:
#   make help
#   make build
#   make run ARGS="--foo bar"
#   # (run-py target removed; enable features via FEATURES var if needed)
#   make test TEST=core                     # run tests whose names match 'core'
#   make clippy STRICT=1                    # deny warnings
#   make watch-run                          # auto-rebuild & run (requires cargo-watch)
#   make fmt lint                           # format + clippy

CARGO ?= cargo
# Extra feature flags can be passed via FEATURES, e.g. FEATURES="--features parsec-ui/python-classifier"
FEATURES ?=
# Additional args passed to the binary after '--'
ARGS ?=
# If set (STRICT=1), clippy will deny warnings
STRICT ?=

# Detect number of CPUs for potential parallel builds (fallback to 4)
JOBS ?= $(shell nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)

# Binary (UI crate)
UI_CRATE = parsec-ui

# Internal helpers
ifdef STRICT
CLIPPY_WARN=-D warnings
else
CLIPPY_WARN=
endif

.PHONY: help build run test fmt clippy lint watch-run watch-test clean doc release install uninstall tidy tree

help: ## Show this help
	@echo "Available targets:" && \
	grep -E '^[a-zA-Z0-9_.-]+:.*?## ' $(MAKEFILE_LIST) | \
	sed -E 's/:.*?## /\t/' | sort

build: ## Build all workspace crates (debug)
	$(CARGO) build --workspace $(FEATURES) -j $(JOBS)

release: ## Build optimized release artifacts
	$(CARGO) build --workspace --release $(FEATURES) -j $(JOBS)

run: ## Run the UI binary (debug)
	$(CARGO) run -p $(UI_CRATE) $(FEATURES) -- $(ARGS)

test: ## Run tests (optionally TEST=name substring)
ifeq ($(TEST),)
	$(CARGO) test --workspace $(FEATURES)
else
	$(CARGO) test --workspace $(FEATURES) -- $(TEST)
endif

fmt: ## Format all code (rustfmt)
	$(CARGO) fmt --all

clippy: ## Run clippy (STRICT=1 to deny warnings)
	$(CARGO) clippy --workspace --all-targets $(FEATURES) -- $(CLIPPY_WARN)

lint: fmt clippy ## Format then lint

watch-run: ## Re-run UI on changes (needs cargo-watch)
	@command -v cargo-watch >/dev/null || { echo "cargo-watch not installed. Install with: cargo install cargo-watch"; exit 1; }
	cargo watch -x "run -p $(UI_CRATE) $(FEATURES) -- $(ARGS)"

watch-test: ## Re-run tests on change (needs cargo-watch)
	@command -v cargo-watch >/dev/null || { echo "cargo-watch not installed. Install with: cargo install cargo-watch"; exit 1; }
	cargo watch -x "test --workspace $(FEATURES)"

doc: ## Build docs (open with BROWSER=1)
	$(CARGO) doc --workspace --no-deps $(FEATURES)
ifdef BROWSER
	@xdg-open target/doc/parsec_core/index.html 2>/dev/null || true
endif

install: ## Install UI binary to ~/.cargo/bin
	$(CARGO) install --path crates/ui $(FEATURES)

uninstall: ## Uninstall UI binary
	$(CARGO) uninstall $(UI_CRATE) || true

clean: ## Remove target directory
	$(CARGO) clean

tidy: ## Format + clippy strict + doc
	$(MAKE) fmt && $(MAKE) clippy STRICT=1 && $(MAKE) doc

tree: ## Show dependency tree (needs cargo-tree)
	@command -v cargo-tree >/dev/null || { echo "cargo-tree not installed. Install with: cargo install cargo-tree"; exit 1; }
	cargo tree -e features $(FEATURES)

# Convenience alias
default: run
