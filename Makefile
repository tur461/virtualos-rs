# Workspace name (used for tagging, etc.)
PROJECT   := vitualos_rs

# Paths
BPF_CRATE := ebpf
BPF_TARGET := bpfel-unknown-none
LINUX_TARGET := x86_64-unknown-linux-gnu
OUT_DIR   := target/$(LINUX_TARGET)/release

# Default: build the Linux release binaries
.PHONY: all
all: linux-bpf linux

# --- eBPF bytecode ---
# The BPF object can be built directly on macOS because it targets a no‑std environment.
.PHONY: linux-bpf
linux-bpf:
	cargo +nightly build -p $(BPF_CRATE) --target $(BPF_TARGET) -Z build-std=core
	@echo "eBPF object ready at target/$(BPF_TARGET)/debug/$(BPF_CRATE).o"

# --- Linux binaries ---
.PHONY: linux
linux:
	cross build --target $(LINUX_TARGET) --release -p cli -p daemon
	@echo "Linux binaries: $(OUT_DIR)/virtualos and $(OUT_DIR)/virtualos-daemon"

# --- Clean everything ---
.PHONY: clean
clean:
	cargo clean
	cross clean

# --- Run tests on the Linux target (requires cross) ---
.PHONY: test
test:
	cross test --target $(LINUX_TARGET) --workspace

# --- Create a tarball of the binaries for deployment ---
.PHONY: dist
dist: linux
	tar -czf $(PROJECT)-linux-amd64.tar.gz -C $(OUT_DIR) vitualos virtualos-daemon
	@echo "Created $(PROJECT)-linux-amd64.tar.gz"

# --- Help ---
.PHONY: help
help:
	@echo "Usage:"
	@echo "  make all          Build eBPF and Linux binaries"
	@echo "  make linux        Build only the Linux binaries (requires eBPF object built first)"
	@echo "  make linux-bpf    Build only the eBPF bytecode"
	@echo "  make clean        Clean all build artifacts"
	@echo "  make test         Run tests on the Linux target"
	@echo "  make dist         Create a release tarball"
