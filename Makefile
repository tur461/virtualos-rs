# ============================================================
# VirtualOS-RS Build
# ============================================================

PROJECT := virtualos_rs

# Usage:
#   make
#   make BUILD_PROFILE=release
#
# Supported:
#   debug
#   release
BUILD_PROFILE ?= debug

# Map project profile names to Cargo profile names.
ifeq ($(BUILD_PROFILE),debug)
CARGO_PROFILE := dev
else ifeq ($(BUILD_PROFILE),release)
CARGO_PROFILE := release
else
$(error BUILD_PROFILE must be 'debug' or 'release')
endif

# ------------------------------------------------------------
# Targets
# ------------------------------------------------------------

LINUX_TARGET := x86_64-unknown-linux-gnu
BPF_TARGET   := bpfel-unknown-none
BPF_CRATE    := ebpf_probes

LINUX_OUT_DIR := target/$(LINUX_TARGET)/$(BUILD_PROFILE)
BPF_OUT_DIR   := target/$(BPF_TARGET)/$(BUILD_PROFILE)

# ------------------------------------------------------------
# Tools
# ------------------------------------------------------------

CARGO      := cargo
RUSTC      := rustc
BPF_LINKER := bpf-linker

# ------------------------------------------------------------
# Default
# ------------------------------------------------------------

.PHONY: all
all: build

# ------------------------------------------------------------
# Check
# ------------------------------------------------------------

.PHONY: check
check:
	@echo "== Rust =="
	@$(RUSTC) --version
	@$(CARGO) --version
	@echo
	@echo "== Nightly Rust =="
	@$(RUSTC) +nightly --version
	@echo
	@echo "== bpf-linker =="
	@command -v $(BPF_LINKER)
	@$(BPF_LINKER) --version
	@echo
	@echo "== BPF target =="
	@$(RUSTC) +nightly --print target-list | grep -E '^bpf(el|eb)-unknown-none$$'
	@echo
	@echo "== Build profile =="
	@echo "Project profile : $(BUILD_PROFILE)"
	@echo "Cargo profile   : $(CARGO_PROFILE)"

# ------------------------------------------------------------
# eBPF
# ------------------------------------------------------------

.PHONY: bpf
bpf:
	@echo "Building eBPF: $(BPF_CRATE)"
	$(CARGO) +nightly build \
		-p $(BPF_CRATE) \
		--target $(BPF_TARGET) \
		--profile $(CARGO_PROFILE) \
		-Z build-std=core \
		-Z build-std-features=compiler-builtins-mem

	@echo
	@echo "eBPF output:"
	@find $(BPF_OUT_DIR) -maxdepth 1 -type f \
		-name '$(BPF_CRATE)*' \
		-print

# ------------------------------------------------------------
# Linux userspace
# ------------------------------------------------------------

.PHONY: linux
linux:
	@echo "Building Linux userspace"

	$(CARGO) build \
		--target $(LINUX_TARGET) \
		--profile $(CARGO_PROFILE) \
		-p cgroups \
		-p cli \
		-p daemon \
		-p ebpf \
		-p engine \
		-p logging \
		-p monitoring \
		-p network \
		-p proto \
		-p storage \
		-p virtualization

	@echo
	@echo "Linux output:"
	@find $(LINUX_OUT_DIR) -maxdepth 1 -type f \
		\( -name 'virtualos*' -o -name 'daemon*' -o -name 'lib*' \) \
		-print

# ------------------------------------------------------------
# Everything
# ------------------------------------------------------------

.PHONY: build
build: bpf linux

# ------------------------------------------------------------
# Release
# ------------------------------------------------------------

.PHONY: release
release:
	$(MAKE) BUILD_PROFILE=release build

# ------------------------------------------------------------
# Debug
# ------------------------------------------------------------

.PHONY: debug
debug:
	$(MAKE) BUILD_PROFILE=debug build

# ------------------------------------------------------------
# Clean
# ------------------------------------------------------------

.PHONY: clean
clean:
	$(CARGO) clean

# ------------------------------------------------------------
# Test
# ------------------------------------------------------------

.PHONY: test
test:
	$(CARGO) test \
		--target $(LINUX_TARGET) \
		--workspace

# ------------------------------------------------------------
# Distribution
# ------------------------------------------------------------

.PHONY: dist
dist: linux
	@mkdir -p dist

	tar -czf \
		$(PROJECT)-linux-amd64.tar.gz \
		-C $(LINUX_OUT_DIR) \
		virtualos \
		virtualos-daemon

	@echo
	@echo "Created:"
	@echo "  dist/$(PROJECT)-linux-amd64.tar.gz"

# ------------------------------------------------------------
# Help
# ------------------------------------------------------------

.PHONY: help
help:
	@echo "VirtualOS-RS"
	@echo
	@echo "Usage:"
	@echo "  make                    Build debug eBPF + Linux"
	@echo "  make debug              Build debug"
	@echo "  make release            Build release"
	@echo "  make bpf                Build eBPF only"
	@echo "  make linux              Build Linux userspace only"
	@echo "  make test               Run tests"
	@echo "  make clean              Clean target/"
	@echo "  make dist               Create Linux distribution archive"
	@echo "  make check              Check build environment"
	@echo
	@echo "Explicit profile:"
	@echo "  make BUILD_PROFILE=debug"
	@echo "  make BUILD_PROFILE=release"

