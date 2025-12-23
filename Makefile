# InferaDB Rust SDK Makefile
# Run `make help` to see available targets

.PHONY: help build test test-unit test-integration test-all check clean
.PHONY: coverage coverage-html
.PHONY: fmt fmt-check lint clippy doc doc-open doc-check
.PHONY: proto
.PHONY: setup ci

# Default target
.DEFAULT_GOAL := help

# Colors for output
BLUE := \033[34m
GREEN := \033[32m
YELLOW := \033[33m
RESET := \033[0m

#───────────────────────────────────────────────────────────────────────────────
# Help
#───────────────────────────────────────────────────────────────────────────────

help: ## Show this help message
	@echo "$(BLUE)InferaDB Rust SDK$(RESET)"
	@echo ""
	@echo "$(GREEN)Usage:$(RESET) make [target]"
	@echo ""
	@echo "$(GREEN)Build & Test:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '^(build|test|clean)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Code Coverage:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '^coverage' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Code Quality:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '^(fmt|lint|clippy|check)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Documentation:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '^doc' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Code Generation:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '^proto' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Other:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '^(setup|ci)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(RESET) %s\n", $$1, $$2}'

#───────────────────────────────────────────────────────────────────────────────
# Build & Test
#───────────────────────────────────────────────────────────────────────────────

build: ## Build the project
	cargo build --workspace --all-features

test: test-unit ## Run unit tests (alias for test-unit)

test-unit: ## Run unit tests only
	cargo test --lib --features insecure

test-integration: ## Run integration tests (requires dev environment)
	cargo test --test integration --features insecure

test-all: ## Run all tests (unit + integration)
	cargo test --lib --test integration --features insecure

clean: ## Clean build artifacts
	cargo clean
	rm -rf target/doc target/llvm-cov

#───────────────────────────────────────────────────────────────────────────────
# Code Coverage
#───────────────────────────────────────────────────────────────────────────────

coverage: ## Run tests with coverage report
	cargo llvm-cov --lib --features insecure --ignore-filename-regex 'proto|inferadb\.v1'

coverage-html: ## Generate HTML coverage report
	cargo llvm-cov --lib --features insecure --ignore-filename-regex 'proto|inferadb\.v1' --html
	@echo "$(GREEN)Report: target/llvm-cov/html/index.html$(RESET)"

#───────────────────────────────────────────────────────────────────────────────
# Code Quality
#───────────────────────────────────────────────────────────────────────────────

fmt: ## Format code with rustfmt
	cargo +nightly fmt --all

fmt-check: ## Check code formatting
	cargo +nightly fmt --all -- --check

clippy: ## Run clippy linter
	cargo clippy --workspace --all-targets -- -D warnings

lint: clippy ## Alias for clippy

check: fmt-check clippy ## Run all code checks (format + clippy)
	@echo "$(GREEN)All checks passed!$(RESET)"

#───────────────────────────────────────────────────────────────────────────────
# Documentation
#───────────────────────────────────────────────────────────────────────────────

doc: ## Build documentation
	cargo doc --workspace --no-deps

doc-open: ## Build and open documentation in browser
	cargo doc --workspace --no-deps --open

doc-check: ## Check documentation for warnings/errors
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
	@echo "$(GREEN)Documentation check passed!$(RESET)"

#───────────────────────────────────────────────────────────────────────────────
# Code Generation
#───────────────────────────────────────────────────────────────────────────────

proto: ## Regenerate protobuf code and format
	@echo "$(BLUE)Regenerating protobuf code...$(RESET)"
	@touch proto/inferadb.proto
	cargo build --features grpc
	$(MAKE) fmt
	@echo "$(GREEN)Protobuf code regenerated and formatted!$(RESET)"

#───────────────────────────────────────────────────────────────────────────────
# Setup & CI
#───────────────────────────────────────────────────────────────────────────────

setup: ## Install development tools
	mise trust
	mise install
	rustup component add rustfmt clippy
	rustup toolchain install nightly --component rustfmt
	@echo "$(GREEN)Setup complete!$(RESET)"

ci: fmt-check clippy test doc-check ## CI pipeline checks
	@echo "$(GREEN)CI checks passed!$(RESET)"
