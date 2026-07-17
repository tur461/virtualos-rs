#!/bin/bash
# Enable cgroup v2 controllers for docklet (idempotent)
CGROUP_ROOT="/sys/fs/cgroup"
SUBCTRL="$CGROUP_ROOT/cgroup.subtree_control"
REQUIRED="cpu memory pids"

if [ ! -f "$SUBCTRL" ]; then
  echo "cgroup v2 not mounted at $CGROUP_ROOT"
  exit 1
fi

current=$(cat "$SUBCTRL")
for ctrl in $REQUIRED; do
  if ! echo "$current" | grep -qw "$ctrl"; then
    echo "+$ctrl" | sudo tee "$SUBCTRL" >/dev/null
  fi
done
