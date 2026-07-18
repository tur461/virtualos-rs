# Virtualos-rs

A container platform built from scratch in Rust – bringing together Linux namespaces, cgroups, OverlayFS, and a gRPC daemon, with upcoming eBPF observability and KVM-based microVM support.

**Status:** Phases 0–9 complete. The core container runtime works, including image pulling, isolated containers, resource limits, networking, a lifecycle manager, and an optional daemon with gRPC API.

## Features (so far)

- **OCI Image Support** – Pull images from Docker Hub (or any OCI registry), verify digests, and unpack layers into a content-addressable store.
- **Layered Filesystem** – OverlayFS mounts combine read-only image layers with a writable upper layer, providing copy-on-write container roots.
- **Strong Isolation** – PID, UTS, mount, and network namespaces give each container its own process space, hostname, filesystem, and network stack.
- **Resource Controls** – cgroup v2 limits for memory and CPU (e.g., `--memory 128m --cpus 1.5`).
- **Networking** – A bridge (`virtualos0`) with veth pairs connects containers to the host and the internet via NAT.
- **Lifecycle Management** – Create, start, stop, delete, and list containers. State is persisted on disk.
- **Foreground & Background** – Run containers interactively (with Ctrl-C forwarding) or detached.
- **gRPC Daemon** – Long-lived `virtualos-rs-daemon` process listening on a Unix socket, serving a gRPC API. The CLI can act as a client or fall back to direct local operation.
- **Clean CLI** – Subcommands like `pull`, `create`, `start`, `stop`, `rm`, `ps`, `run`, and `network-init`.

## Architecture

The project is a Cargo workspace composed of several crates, each with a clear responsibility:

| Crate | Purpose |
|--------|---------|
| `cli` | The `virtualos-rs` binary – CLI parsing and user interaction |
| `engine` | Container lifecycle, image pulling, overlay preparation |
| `storage` | Content-addressable layer store and OverlayFS helpers |
| `network` | Bridge, veth, NAT setup (calls external `ip`/`iptables`) |
| `cgroups` | cgroup v2 management (memory, CPU limits) |
| `monitoring` | *Planned* – Prometheus metrics endpoint |
| `logging` | *Planned* – Structured logging (`tracing`) |
| `ebpf` | *Planned* – eBPF probes via `aya` |
| `virtualization` | *Planned* – KVM-based microVM runner |
| `daemon` | The `virtualos-rs-daemon` binary – gRPC server |
| `proto` | Protobuf definitions for the gRPC API |

## Prerequisites

- Linux kernel **5.x+** (x86_64) with cgroup v2, overlay, and `br_netfilter` modules.
- Rust **nightly** toolchain (edition 2024). Install via `rustup toolchain install nightly`.
- `protoc` (Protocol Buffers compiler) – `sudo apt install protobuf-compiler` on Debian/Ubuntu.
- Root access (`sudo`) – required for namespace creation, mounts, and networking.
- `iptables` and `iproute2` for networking.

## Building

```bash
# Clone the repository
git clone <repo-url>
cd virtualos-rs

# Build all binaries
cargo +nightly build --release
```

The two binaries are:

- `target/release/virtualos-rs` – the CLI
- `target/release/virtualos-rs-daemon` – the gRPC daemon

## Quick Start (Local Mode)

All commands below must be run as root (or with `sudo`) when using direct mode.

### 1. Initialise the network (once per host)

```bash
sudo virtualos-rs network-init
```

### 2. Pull an image

```bash
sudo virtualos-rs pull alpine:latest --store-dir /var/lib/virtualos-rs/store
```

### 3. Run a container

```bash
# Foreground (attached)
sudo virtualos-rs run alpine sh -c "echo 'Hello from container'"

# Detached
sudo virtualos-rs run -d alpine sleep 30

# With memory and CPU limits
sudo virtualos-rs run --memory 64m --cpus 0.5 alpine stress --vm 1 --vm-bytes 50M
```

### 4. Manage containers

```bash
sudo virtualos-rs ps                          # list containers
sudo virtualos-rs stop <container-id>
sudo virtualos-rs rm <container-id>           # remove (must be stopped)
sudo virtualos-rs rm -f <container-id>        # force remove (stops first)
```

## Using the Daemon (Client/Server Mode)

### Start the daemon (as root)

```bash
sudo virtualos-rs-daemon
```

### Use the CLI as a client (any user with access to the socket)

```bash
# Make sure the socket is accessible (adjust permissions as needed)
sudo chmod 0666 /var/run/virtualos-rs.sock

# Now the CLI will automatically connect to the daemon
virtualos-rs pull alpine:latest
virtualos-rs run -d alpine sleep 10
virtualos-rs ps
```

When the daemon is running, the CLI uses gRPC over the Unix socket. If the daemon is not running, the CLI falls back to direct (local) mode and requires root privileges.

## Project Status & Roadmap

| Phase | Feature | Status |
|------:|---------|:------:|
| 0 | Workspace setup | ✅ Done |
| 1 | Namespace isolation (PID, UTS) | ✅ Done |
| 2 | `pivot_root` and root filesystem | ✅ Done |
| 3 | Image pulling & unpacking | ✅ Done |
| 4 | OverlayFS | ✅ Done |
| 5 | Container lifecycle manager | ✅ Done |
| 6 | Cgroups (memory & CPU limits) | ✅ Done |
| 7 | Networking (bridge + NAT) | ✅ Done |
| 8 | CLI polish & error handling | ✅ Done |
| 9 | Daemon & gRPC API | ✅ Done |
| 10 | Structured logging (`tracing`) | ⏳ Next |
| 11 | Prometheus monitoring | 🗓️ Planned |
| 12 | eBPF tracing & security | 🗓️ Planned |
| 13 | KVM-based microVM isolation | 🗓️ Planned |
| 14 | End-to-end integration & tests | 🗓️ Planned |

## Contributing

This is an educational project built step-by-step. Contributions, issues, and feedback are welcome! Please see the phased plan in the repository for the current development stage.

## License

This project is licensed under the [MIT License](LICENSE).
