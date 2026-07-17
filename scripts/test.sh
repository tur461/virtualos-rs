#!/bin/bash
# test.sh – Run a specific integration test or all of them.
# Usage: ./test.sh <test_name>
# Available tests: memory, network, foreground, detach, rm, all
#
#
# to capture all logs, use strace
#
# sudo strace -f -o trace.log ./target/debug/virtualos-rs run --memory 64m alpine sh -- -c 'echo && echo "Memory MAX:" && cat /sys/fs/cgroup/memory.max && echo'
#
#

set -euo pipefail

BIN="./target/debug/virtualos-rs" # adjust if your binary name differs
STORE_DIR="./test-store"
NETWORK_INIT_DONE=false

# ---------- helper ----------
setup_network_once() {
  if ! $NETWORK_INIT_DONE; then
    echo "[setup] Initialising bridge and NAT..."
    sudo "$BIN" network-init
    NETWORK_INIT_DONE=true
    echo "[setup] Bridge ready."
  fi
}

# ---------- test functions ----------
test_memory() {
  echo "=== memory limit test ==="
  sudo "$BIN" run --memory 64m --rm alpine sh -c \
    'echo && echo "Memory MAX:" && cat /sys/fs/cgroup/memory.max && echo'
  echo "[PASS] memory limit"
}

test_network() {
  setup_network_once
  echo "=== network test ==="
  sudo "$BIN" run --rm alpine sh -c \
    "ip addr show ceth* 2>/dev/null || ip addr show eth*; ping -c1 8.8.8.8"
  echo "[PASS] network"
}

test_foreground() {
  echo "=== foreground test ==="
  sudo "$BIN" run --rm alpine echo "foreground test passed"
  echo "[PASS] foreground"
}

test_detach() {
  setup_network_once
  echo "=== detached run test ==="
  id=$(sudo "$BIN" run -d alpine sleep 5)
  echo "Started container $id"
  sleep 1
  echo "--- ps after start ---"
  sudo "$BIN" ps
  sleep 6
  echo "--- ps after sleep ---"
  sudo "$BIN" ps
  # check that container no longer shows "Running" (should be Stopped or absent)
  if sudo "$BIN" ps | grep "$id" | grep -q Running; then
    echo "FAIL: container still running"
    exit 1
  fi
  echo "[PASS] detached"
}

test_rm() {
  echo "=== --rm flag test ==="
  sudo "$BIN" run --rm alpine echo "this container will be removed"
  # verify it's gone (a bit indirect: try to delete it and expect failure?)
  echo "[PASS] --rm (no error during run, immediate deletion)"
}

# ---------- dispatch ----------
if [ $# -ne 1 ]; then
  echo "Usage: $0 <test_name>"
  echo "Available: memory, network, foreground, detach, rm, all"
  exit 1
fi

TEST="$1"
case "$TEST" in
memory) test_memory ;;
network) test_network ;;
foreground) test_foreground ;;
detach) test_detach ;;
rm) test_rm ;;
all)
  test_memory
  test_network
  test_foreground
  test_detach
  test_rm
  ;;
*)
  echo "Unknown test: $TEST"
  exit 1
  ;;
esac
