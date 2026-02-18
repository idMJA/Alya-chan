SHELL := /bin/bash
CARGO ?= cargo

.PHONY: help start start-release format fmt-check lint check build release test clean ci

.DEFAULT_GOAL := help

help:
	@echo "Alya-chan — available make targets:"
	@echo "  start           Run (debug)       -> cargo run"
	@echo "  start-release   Run (release)     -> cargo run --release"
	@echo "  format          Format source      -> cargo fmt --all"
	@echo "  fmt-check       Check formatting   -> cargo fmt -- --check"
	@echo "  lint            Check & lint       -> cargo check && cargo clippy --all -- -D warnings"
	@echo "  check           cargo check"
	@echo "  build           cargo build"
	@echo "  release         cargo build --release"
	@echo "  test            cargo test"
	@echo "  clean           cargo clean"
	@echo "  ci              fmt-check + lint + test"

start:
	$(CARGO) run

start-release:
	$(CARGO) run --release

format:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt -- --check

lint:
	$(CARGO) check && $(CARGO) clippy --all -- -D warnings

check:
	$(CARGO) check

build:
	$(CARGO) build

release:
	$(CARGO) build --release

test:
	$(CARGO) test

clean:
	$(CARGO) clean

ci: fmt-check lint test
	@echo "CI checks passed"
