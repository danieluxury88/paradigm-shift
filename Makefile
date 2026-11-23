# Paradigm Shift: common tasks. Run `make help` for the list.
SHELL := /bin/bash
export PATH := $(HOME)/.cargo/bin:$(PATH)

ZIP := build/paradigm-shift-web.zip

.PHONY: help run brp test check web serve itch clean

help: ## List the available targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  %-8s %s\n", $$1, $$2}'

run: ## Play natively in a window (fast incremental builds)
	cargo run --features dev

brp: ## Play natively with remote control for coding agents (BRP on 127.0.0.1:15702)
	cargo run --features dev,brp

test: ## Run the unit tests (level files are validated here)
	cargo test --features dev

check: ## Format check, lints and tests
	cargo fmt --check
	cargo clippy --features dev -- -D warnings
	cargo test --features dev

serve: ## Play the web build locally with live reload (http://127.0.0.1:8080)
	trunk serve

web: ## Build the optimized web version and zip it for itch.io
	trunk build --release --public-url ./
	@mkdir -p build && rm -f $(ZIP)
	cd dist && zip -qr ../$(ZIP) .
	@echo "Upload $(ZIP) to itch.io as an HTML game (\"This file will be played in the browser\")."
	@du -h $(ZIP)

clean: ## Remove build output
	cargo clean
	rm -rf dist build
