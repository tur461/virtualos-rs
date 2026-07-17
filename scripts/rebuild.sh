#!/bin/sh

echo "Removing cgroup sub directory.."
sudo rmdir /sys/fs/cgroup/docklet/* 2>/dev/null
sudo rmdir /sys/fs/cgroup/docklet 2>/dev/null

echo "Removing old target directory.."
sudo rm -rf target

echo "Rebuilding whole workspace.."
cargo build --workspace 1>/dev/null

echo "done."
