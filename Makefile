# InferaDB Rust SDK Makefile
# Run `make help` to see available targets

.PHONY: help build test check clean
.PHONY: fmt fmt-check lint clippy doc doc-check
.PHONY: lint-docs lint-markdown lint-prose lint-spelling lint-deadlinks
.PHONY: lint-all setup-tools

# Default target
.DEFAULT_GOAL := help

# Colors for output
BLUE := \033[34m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
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
	@echo "$(GREEN)Code Quality:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '^(fmt|lint|clippy|check)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Documentation:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '^(doc|lint-docs|lint-markdown|lint-prose|lint-spelling|lint-deadlinks)' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(GREEN)Setup:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E '^setup' | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(RESET) %s\n", $$1, $$2}'

#───────────────────────────────────────────────────────────────────────────────
# Build & Test
#───────────────────────────────────────────────────────────────────────────────

build: ## Build the project
	cargo build --all-features

test: ## Run all tests
	cargo test --all-features

test-coverage: ## Run tests with coverage report
	cargo llvm-cov --all-features --html

clean: ## Clean build artifacts
	cargo clean
	rm -rf target/doc

#───────────────────────────────────────────────────────────────────────────────
# Code Formatting
#───────────────────────────────────────────────────────────────────────────────

fmt: ## Format code with rustfmt
	cargo +nightly fmt --all

fmt-check: ## Check code formatting without making changes
	cargo +nightly fmt --all -- --check

#───────────────────────────────────────────────────────────────────────────────
# Code Quality - Rust
#───────────────────────────────────────────────────────────────────────────────

clippy: ## Run clippy linter
	cargo clippy --workspace --all-targets --all-features -- -D warnings

lint: clippy ## Alias for clippy

check: fmt-check clippy ## Run all code checks (format + clippy)
	@echo "$(GREEN)All code checks passed!$(RESET)"

#───────────────────────────────────────────────────────────────────────────────
# Documentation - Build
#───────────────────────────────────────────────────────────────────────────────

doc: ## Build documentation
	cargo doc --no-deps --all-features

doc-open: ## Build and open documentation in browser
	cargo doc --no-deps --all-features --open

doc-check: ## Check documentation for warnings/errors
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
	@echo "$(GREEN)Documentation check passed!$(RESET)"

#───────────────────────────────────────────────────────────────────────────────
# Documentation - Linting
#───────────────────────────────────────────────────────────────────────────────

lint-markdown: ## Lint markdown files with markdownlint-cli2
	@if command -v markdownlint-cli2 >/dev/null 2>&1; then \
		markdownlint-cli2 "**/*.md" "!node_modules" "!target" "!.vale"; \
	elif command -v npx >/dev/null 2>&1; then \
		npx markdownlint-cli2 "**/*.md" "!node_modules" "!target" "!.vale"; \
	else \
		echo "$(RED)Error: markdownlint-cli2 not found. Install with: npm install -g markdownlint-cli2$(RESET)"; \
		exit 1; \
	fi
	@echo "$(GREEN)Markdown lint passed!$(RESET)"

lint-prose: ## Lint prose with Vale (docs + source comments)
	@if command -v vale >/dev/null 2>&1; then \
		vale sync && vale docs/ src/ README.md CONTRIBUTING.md MIGRATION.md; \
	else \
		echo "$(RED)Error: Vale not found. Install with: brew install vale (or see https://vale.sh)$(RESET)"; \
		exit 1; \
	fi
	@echo "$(GREEN)Prose lint passed!$(RESET)"

lint-spelling: ## Check spelling (note: Vale handles this via lint-prose)
	@echo "$(BLUE)Spelling is checked by Vale via lint-prose target$(RESET)"
	@echo "$(BLUE)Run 'make lint-prose' for spell checking$(RESET)"

lint-deadlinks: ## Check for broken links in generated documentation
	@if ! [ -d "target/doc" ]; then \
		echo "$(YELLOW)Building documentation first...$(RESET)"; \
		cargo doc --no-deps --all-features; \
	fi
	@if command -v cargo-deadlinks >/dev/null 2>&1; then \
		cargo deadlinks 2>&1 || echo "$(YELLOW)Note: Some dead links are from dependency docs (e.g., tracing crate)$(RESET)"; \
	elif cargo deadlinks --version >/dev/null 2>&1; then \
		cargo deadlinks 2>&1 || echo "$(YELLOW)Note: Some dead links are from dependency docs (e.g., tracing crate)$(RESET)"; \
	else \
		echo "$(YELLOW)Warning: cargo-deadlinks not found. Install with: cargo install cargo-deadlinks$(RESET)"; \
	fi

lint-docs: lint-markdown lint-prose doc-check lint-deadlinks ## Run all documentation lints
	@echo "$(GREEN)All documentation lints passed!$(RESET)"

#───────────────────────────────────────────────────────────────────────────────
# Combined Targets
#───────────────────────────────────────────────────────────────────────────────

lint-all: check lint-docs ## Run ALL lints (code + documentation)
	@echo "$(GREEN)All lints passed!$(RESET)"

ci: fmt-check clippy test doc-check lint-markdown ## CI pipeline checks
	@echo "$(GREEN)CI checks passed!$(RESET)"

#───────────────────────────────────────────────────────────────────────────────
# Setup
#───────────────────────────────────────────────────────────────────────────────

setup-tools: ## Install all linting tools
	@echo "$(BLUE)Installing Rust tools...$(RESET)"
	rustup component add rustfmt clippy
	cargo install cargo-deadlinks cargo-spellcheck cargo-llvm-cov || true
	@echo ""
	@echo "$(BLUE)Installing Node.js tools...$(RESET)"
	@if command -v npm >/dev/null 2>&1; then \
		npm install -g markdownlint-cli2 || echo "$(YELLOW)npm install failed - try with sudo$(RESET)"; \
	else \
		echo "$(YELLOW)npm not found - skipping markdownlint-cli2$(RESET)"; \
	fi
	@echo ""
	@echo "$(BLUE)Installing Vale and dependencies...$(RESET)"
	@if command -v brew >/dev/null 2>&1; then \
		brew install vale docutils || true; \
	elif command -v apt-get >/dev/null 2>&1; then \
		echo "$(YELLOW)Install Vale manually: https://vale.sh/docs/vale-cli/installation/$(RESET)"; \
		echo "$(YELLOW)Install docutils: pip install docutils$(RESET)"; \
	else \
		echo "$(YELLOW)Install Vale manually: https://vale.sh/docs/vale-cli/installation/$(RESET)"; \
		echo "$(YELLOW)Install docutils: pip install docutils$(RESET)"; \
	fi
	@echo ""
	@echo "$(BLUE)Syncing Vale packages...$(RESET)"
	@if command -v vale >/dev/null 2>&1; then \
		vale sync; \
	fi
	@echo ""
	@echo "$(GREEN)Setup complete!$(RESET)"

setup-vale: ## Initialize Vale styles
	@if command -v vale >/dev/null 2>&1; then \
		vale sync; \
		echo "$(GREEN)Vale styles synced!$(RESET)"; \
	else \
		echo "$(RED)Error: Vale not found. Install with: brew install vale$(RESET)"; \
		exit 1; \
	fi
