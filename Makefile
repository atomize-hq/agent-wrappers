# Keep only things under ./crates/ (like the Substrate repo Makefile pattern).
CRATES_ALL := $(shell find . -maxdepth 3 -type f -name Cargo.toml -exec dirname {} \; | sort -u)
CRATES := $(filter ./crates/%,$(CRATES_ALL))
CRATES := $(patsubst ./%,%,$(CRATES))

# ---- Tooling (auto-install to reduce host deps) ----
TOKEI_ROOT := $(CURDIR)/target/tools/tokei
TOKEI_BIN := $(TOKEI_ROOT)/bin/tokei
TOKEI_SYSTEM := $(shell command -v tokei 2>/dev/null)
ifeq ($(TOKEI_SYSTEM),)
TOKEI := $(TOKEI_BIN)
else
TOKEI := $(TOKEI_SYSTEM)
endif

CARGO_TOOLS_ROOT := $(CURDIR)/target/tools/cargo-tools
CARGO_TOOLS_BIN := $(CARGO_TOOLS_ROOT)/bin

CRATE_CMD ?= $(TOKEI) .

LOG_ROOT := target/crate-logs
DATE_DIR := $(shell date -u +%m-%-d-%y)
LOG_DIR := $(LOG_ROOT)/$(DATE_DIR)
RUN_TS := $(shell date -u +%Y%m%dT%H%M%SZ)
FINAL_LOG := $(LOG_DIR)/__all-crates.$(RUN_TS).log

# ---- Policy knobs ----
# Hard cap: max Rust "code" LOC per file.
LOC_CAP ?= 700

# Avoid counting generated/archived dirs.
TOKEI_EXCLUDES ?= target audit_pack evidence_runs cli_manifests
TOKEI_EXCLUDE_FLAGS := $(foreach e,$(TOKEI_EXCLUDES),--exclude $(e))
TOKEI_JSON := target/tokei_files.json

# Security checks often need a writable cargo home (avoids advisory-db lock issues in some envs).
SEC_CARGO_HOME := $(CURDIR)/target/security-cargo-home

# cargo-deny checks to run by default (sources/bans remain opt-in via override).
DENY_CHECKS ?= advisories licenses

.PHONY: tokei-all-crates
tokei-all-crates: ensure-tokei
	@mkdir -p "$(LOG_DIR)"
	@echo "Date dir (UTC): $(DATE_DIR)"
	@echo "Run timestamp (UTC): $(RUN_TS)"
	@echo "Log dir: $(LOG_DIR)"
	@echo "CRATES = $(CRATES)"
	@set -e; \
	for d in $(CRATES); do \
	  crate=$$(basename "$$d"); \
	  cmd_tag=$$(printf '%s\n' "$(CRATE_CMD)" | tr ' /' '-_'); \
	  log="$(LOG_DIR)/$${crate}_$${cmd_tag}_$(RUN_TS).log"; \
	  echo "===== BEGIN $$d =====" | tee "$$log"; \
	  (cd "$$d" && $(CRATE_CMD)) 2>&1 | tee -a "$$log"; \
	  echo "===== END $$d =====" | tee -a "$$log"; \
	  echo "" >> "$$log"; \
	done; \
	cat "$(LOG_DIR)"/*_*$$(printf '%s\n' "$(RUN_TS)").log > "$(FINAL_LOG)"; \
	echo "Combined log written to: $(FINAL_LOG)"

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: fmt-check
fmt-check:
	cargo fmt --all -- --check

.PHONY: clippy
clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: check
check:
	cargo check --workspace --all-targets --all-features

.PHONY: test
test:
	cargo test --workspace --all-targets --all-features

.PHONY: loc-check
loc-check: ensure-tokei
	@mkdir -p target
	@$(TOKEI) . --files --output json $(TOKEI_EXCLUDE_FLAGS) > $(TOKEI_JSON)
	@printf '%s\n' \
	  'import json, os, sys' \
	  'cap = int(os.environ.get("LOC_CAP","700"))' \
	  'path = os.environ.get("TOKEI_JSON","target/tokei_files.json")' \
	  'with open(path, "r", encoding="utf-8") as f:' \
	  '    data = json.load(f)' \
	  'rust = None' \
	  'for k, v in data.items():' \
	  '    if k.lower() == "rust":' \
	  '        rust = v' \
	  '        break' \
	  'if rust is None:' \
	  '    print("loc-check: no Rust section found in tokei json; skipping")' \
	  '    sys.exit(0)' \
	  'reports = rust.get("reports") or []' \
	  'off = []' \
	  'for r in reports:' \
	  '    name = r.get("name") or r.get("path") or r.get("filename")' \
	  '    code = r.get("code")' \
	  '    if code is None:' \
	  '        stats = r.get("stats") or {}' \
	  '        code = stats.get("code")' \
	  '    if name and code is not None and int(code) > cap:' \
	  '        off.append((int(code), name))' \
	  'off.sort(reverse=True)' \
	  'if off:' \
	  '    print(f"loc-check: FAIL - Rust file code LOC cap exceeded (cap={cap})")' \
	  '    for code, name in off:' \
	  '        print(f"  {code:>6}  {name}")' \
	  '    sys.exit(1)' \
	  'print(f"loc-check: PASS - no Rust file exceeds {cap} code lines")' \
	| LOC_CAP="$(LOC_CAP)" TOKEI_JSON="$(TOKEI_JSON)" python3 -

.PHONY: ensure-tokei
ensure-tokei:
	@if [ -n "$(TOKEI_SYSTEM)" ]; then \
	  echo "ensure-tokei: using system tokei ($(TOKEI_SYSTEM))"; \
	else \
	  if [ ! -x "$(TOKEI_BIN)" ]; then \
	    echo "ensure-tokei: installing tokei into $(TOKEI_ROOT)"; \
	    mkdir -p "$(TOKEI_ROOT)"; \
	    cargo install tokei --locked --root "$(TOKEI_ROOT)"; \
	  else \
	    echo "ensure-tokei: using cached tokei ($(TOKEI_BIN))"; \
	  fi; \
	fi

.PHONY: ensure-security-tools
ensure-security-tools:
	@mkdir -p "$(CARGO_TOOLS_ROOT)"
	@if ! command -v cargo-audit >/dev/null 2>&1 && [ ! -x "$(CARGO_TOOLS_BIN)/cargo-audit" ]; then \
	  echo "ensure-security-tools: installing cargo-audit into $(CARGO_TOOLS_ROOT)"; \
	  cargo install cargo-audit --locked --root "$(CARGO_TOOLS_ROOT)"; \
	fi
	@if ! command -v cargo-deny >/dev/null 2>&1 && [ ! -x "$(CARGO_TOOLS_BIN)/cargo-deny" ]; then \
	  echo "ensure-security-tools: installing cargo-deny into $(CARGO_TOOLS_ROOT)"; \
	  cargo install cargo-deny --locked --root "$(CARGO_TOOLS_ROOT)"; \
	fi
	@if ! command -v cargo-geiger >/dev/null 2>&1 && [ ! -x "$(CARGO_TOOLS_BIN)/cargo-geiger" ]; then \
	  echo "ensure-security-tools: installing cargo-geiger into $(CARGO_TOOLS_ROOT)"; \
	  cargo install cargo-geiger --locked --root "$(CARGO_TOOLS_ROOT)"; \
	fi

.PHONY: security
security: ensure-security-tools
	@mkdir -p "$(SEC_CARGO_HOME)"
	@echo "## security checks (CARGO_HOME=$(SEC_CARGO_HOME))"
	@PATH="$(CARGO_TOOLS_BIN):$$PATH" CARGO_HOME="$(SEC_CARGO_HOME)" cargo audit
	@set -e; \
	for c in $(DENY_CHECKS); do \
	  echo "cargo deny check $$c"; \
	  PATH="$(CARGO_TOOLS_BIN):$$PATH" CARGO_HOME="$(SEC_CARGO_HOME)" cargo deny check $$c; \
	done

.PHONY: unsafe-report
unsafe-report: ensure-security-tools
	@mkdir -p "$(LOG_DIR)"
	@echo "## unsafe-report (cargo geiger) — informational only"
	@echo "## NOTE: cargo-geiger currently emits parse warnings for some deps; do not treat as hard gate."
	@PATH="$(CARGO_TOOLS_BIN):$$PATH" cargo geiger -p codex 2>&1 | tee "$(LOG_DIR)/geiger_codex_$(RUN_TS).log" || true
	@PATH="$(CARGO_TOOLS_BIN):$$PATH" cargo geiger -p xtask 2>&1 | tee "$(LOG_DIR)/geiger_xtask_$(RUN_TS).log" || true
	@echo "Geiger logs: $(LOG_DIR)/geiger_*_$(RUN_TS).log"

.PHONY: flightcheck
flightcheck:
	@echo "##flightcheck -- must run from repo root"
	@echo "##flightcheck -- must pass for *integ tasks to be considered green"
	$(MAKE) fmt-check
	$(MAKE) clippy
	cargo clean
	$(MAKE) check
	$(MAKE) test
	$(MAKE) loc-check
	$(MAKE) security
	$(MAKE) unsafe-report

.PHONY: preflight
.PHONY: hygiene
hygiene:
	./scripts/check_repo_hygiene.sh

.PHONY: preflight
preflight: hygiene flightcheck

.PHONY: adr-check
adr-check:
	@test -n "$(ADR)" || (echo "usage: make adr-check ADR=docs/adr/0009-....md" && exit 2)
	@python3 scripts/adr_hash.py "$(ADR)" >/dev/null

.PHONY: adr-fix
adr-fix:
	@test -n "$(ADR)" || (echo "usage: make adr-fix ADR=docs/adr/0009-....md" && exit 2)
	@python3 scripts/adr_hash.py --fix "$(ADR)" >/dev/null
