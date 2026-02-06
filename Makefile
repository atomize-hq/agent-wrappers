# Keep only things under ./crates/ (like the Substrate repo Makefile pattern).
CRATES_ALL := $(shell find . -maxdepth 3 -type f -name Cargo.toml -exec dirname {} \; | sort -u)
CRATES := $(filter ./crates/%,$(CRATES_ALL))
CRATES := $(patsubst ./%,%,$(CRATES))

CRATE_CMD ?= tokei .

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
tokei-all-crates:
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
loc-check:
	@mkdir -p target
	@tokei . --files --output json $(TOKEI_EXCLUDE_FLAGS) > $(TOKEI_JSON)
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

.PHONY: security
security:
	@mkdir -p "$(SEC_CARGO_HOME)"
	@echo "## security checks (CARGO_HOME=$(SEC_CARGO_HOME))"
	@CARGO_HOME="$(SEC_CARGO_HOME)" cargo audit
	@set -e; \
	for c in $(DENY_CHECKS); do \
	  echo "cargo deny check $$c"; \
	  CARGO_HOME="$(SEC_CARGO_HOME)" cargo deny check $$c; \
	done

.PHONY: unsafe-report
unsafe-report:
	@mkdir -p "$(LOG_DIR)"
	@echo "## unsafe-report (cargo geiger) — informational only"
	@echo "## NOTE: cargo-geiger currently emits parse warnings for some deps; do not treat as hard gate."
	@cargo geiger -p codex 2>&1 | tee "$(LOG_DIR)/geiger_codex_$(RUN_TS).log" || true
	@cargo geiger -p xtask 2>&1 | tee "$(LOG_DIR)/geiger_xtask_$(RUN_TS).log" || true
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
