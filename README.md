# Virtualos-rs

A container and virtualization platform built from scratch in Rust, combining Linux namespaces, cgroups v2, OverlayFS, OCI image handling, container networking, gRPC management, structured logging, Prometheus-compatible monitoring, eBPF observability, and KVM-based virtualization components.

**Status:** Core project implementation complete. The major runtime, storage, networking, lifecycle, daemon, observability, eBPF, and virtualization components are implemented. The remaining work is primarily **test coverage, end-to-end validation, and real-environment testing**.

> **Project maturity:** Virtualos-rs is a functional systems-engineering and learning project. It is not yet positioned as a production replacement for mature runtimes such as containerd, CRI-O, Docker, or a full-featured VMM. Contributions that improve correctness, portability, security, testing, and operational robustness are especially welcome.

## Features

### Container Runtime

- **OCI Image Support** – Pull images by reference from Docker Hub and OCI-compatible registries, authenticate when required, verify layer content, and unpack image layers.
- **Content-Addressable Storage** – Store image layers using their content digests and reuse already-present layers.
- **Layered Filesystem** – OverlayFS combines read-only image layers with a writable upper layer to provide copy-on-write container filesystems.
- **Linux Namespace Isolation** – PID, UTS, mount, and network namespaces provide process, hostname, filesystem, and network isolation.
- **Root Filesystem Setup** – Container root filesystems are prepared from image layers and isolated using Linux filesystem primitives.
- **Resource Controls** – cgroup v2 support for CPU and memory limits.
- **Container Lifecycle Management** – Create, start, stop, remove, force-remove, and list containers with persisted state.
- **Foreground & Detached Execution** – Run containers interactively or in the background with lifecycle and signal handling.
- **Container Networking** – Linux bridge and veth-based networking with address configuration, routing, and NAT support.
- **CLI** – A dedicated command-line interface for image, container, and network management.

### Daemon & API

- **gRPC Daemon** – A long-running `virtualos-rs-daemon` exposes container operations through a gRPC API over a Unix domain socket.
- **CLI Client Mode** – The CLI can communicate with the daemon when available and can operate directly against the local runtime when the daemon is not being used.
- **Protocol Definitions** – Protobuf definitions are maintained in the dedicated `proto` crate.

### Observability

- **Structured Logging** – Centralized logging based on Rust's `tracing` ecosystem.
- **Monitoring** – Monitoring infrastructure is implemented as a dedicated crate for exposing runtime metrics and observability data.
- **eBPF Instrumentation** – Aya-based eBPF infrastructure includes probes covering:
  - Process lifecycle events such as `fork`, `exec`, and `exit`
  - Filesystem events such as `open`, `close`, `rename`, and `unlink`
  - Networking events such as `bind`, `connect`, and `accept`
  - IPv4/IPv6 address extraction
  - Socket metadata and socket operations
  - TCP and UDP-related events
  - Shared event structures and eBPF maps

### Virtualization

- **KVM Virtualization Components** – The `virtualization` crate contains the building blocks for KVM-based virtual machine execution, including:
  - VM configuration
  - vCPU setup
  - guest memory management
  - virtual device handling
  - kernel loading
  - boot configuration
  - VM lifecycle management
  - virtualization-specific error handling

## Architecture

Virtualos-rs is organized as a Cargo workspace. Each crate owns a focused part of the system:

| Crate | Responsibility |
|--------|----------------|
| `cli` | Command-line interface, argument parsing, command dispatch, and user interaction |
| `engine` | Container lifecycle, OCI image handling, root filesystem preparation, and runtime orchestration |
| `storage` | Content-addressable image/layer storage and filesystem storage helpers |
| `network` | Linux bridge, veth, IP configuration, routing, and NAT |
| `cgroups` | cgroup v2 resource management for containers |
| `monitoring` | Runtime monitoring and metrics infrastructure |
| `logging` | Structured application logging using `tracing` |
| `ebpf` | eBPF loader/runtime integration and observability support |
| `ebpf/ebpf-probes` | Kernel-side Aya eBPF probes for process, filesystem, and networking events |
| `virtualization` | KVM-based virtualization primitives and VM construction |
| `daemon` | Long-running gRPC daemon and service implementation |
| `proto` | Protobuf definitions and generated gRPC types |

The repository also contains supporting directories for examples, scripts, the top-level binary entry point, build automation, and runtime data.

## Repository Layout

```text
virtualos-rs/
├── Cargo.toml
├── Cargo.lock
├── crates/
│   ├── cli/
│   ├── engine/
│   ├── storage/
│   ├── network/
│   ├── cgroups/
│   ├── monitoring/
│   ├── logging/
│   ├── ebpf/
│   │   └── ebpf-probes/
│   ├── virtualization/
│   ├── daemon/
│   └── proto/
├── examples/
├── scripts/
├── src/
├── runc/
├── Makefile
├── LICENSE
└── README.md
```

## Prerequisites

Virtualos-rs relies on Linux kernel facilities and therefore requires a Linux host with appropriate privileges.

Recommended environment:

- Linux kernel **5.x or newer**
- x86_64 Linux host
- cgroup v2 enabled
- OverlayFS support
- Linux namespaces enabled
- `br_netfilter` where required by the networking configuration
- Rust **nightly** toolchain
- `protoc` / Protocol Buffers compiler
- `iproute2`
- `iptables` or an equivalent supported packet-filter/NAT configuration
- Root privileges (`sudo`) for namespace, mount, cgroup, networking, eBPF, and virtualization operations
- KVM support for exercising the virtualization components

Install the Rust toolchain with:

```bash
rustup toolchain install nightly
```

On Debian/Ubuntu:

```bash
sudo apt install protobuf-compiler iproute2 iptables
```

> Exact kernel configuration, distribution packages, eBPF support, and KVM availability vary between Linux environments. Real-environment testing is therefore an important remaining project task.

## Building

Clone the repository and build the workspace:

```bash
git clone <repo-url>
cd virtualos-rs

cargo +nightly build --release
```

The workspace contains the main CLI and daemon binaries:

```text
target/release/virtualos-rs
target/release/virtualos-rs-daemon
```

For development builds:

```bash
cargo +nightly build
```

## Quick Start

> Most runtime operations require root privileges because Virtualos-rs directly manipulates namespaces, mounts, cgroups, networking, and other privileged Linux resources.

### 1. Initialise networking

```bash
sudo virtualos-rs network-init
```

### 2. Pull an image

```bash
sudo virtualos-rs pull alpine:latest \
  --store-dir /var/lib/virtualos-rs/store
```

### 3. Run a container

Foreground execution:

```bash
sudo virtualos-rs run alpine sh -c "echo 'Hello from container'"
```

Detached execution:

```bash
sudo virtualos-rs run -d alpine sleep 30
```

With resource limits:

```bash
sudo virtualos-rs run \
  --memory 64m \
  --cpus 0.5 \
  alpine \
  stress --vm 1 --vm-bytes 50M
```

### 4. Inspect and manage containers

```bash
sudo virtualos-rs ps

sudo virtualos-rs stop <container-id>

sudo virtualos-rs rm <container-id>
```

Force removal:

```bash
sudo virtualos-rs rm -f <container-id>
```

## Daemon / Client Mode

Start the daemon:

```bash
sudo virtualos-rs-daemon
```

The daemon exposes the gRPC API through a Unix domain socket.

The CLI can then be used as a client:

```bash
virtualos-rs pull alpine:latest
virtualos-rs run -d alpine sleep 10
virtualos-rs ps
```

Socket permissions should be configured according to the deployment's security requirements. Avoid globally writable socket permissions in production environments; use an appropriate Unix group or service-account policy instead.

When the daemon is unavailable, the CLI can fall back to direct local operation where supported.

## Development Status

The original phased implementation plan has now been completed through the major runtime and systems components.

| Area | Status |
|------|:------:|
| Cargo workspace and project structure | ✅ Complete |
| Linux PID/UTS/mount/network namespace isolation | ✅ Complete |
| Root filesystem and filesystem isolation | ✅ Complete |
| OCI image pulling | ✅ Complete |
| Image layer verification and unpacking | ✅ Complete |
| Content-addressable storage | ✅ Complete |
| OverlayFS container filesystem | ✅ Complete |
| Container lifecycle management | ✅ Complete |
| cgroup v2 CPU and memory controls | ✅ Complete |
| Bridge/veth container networking | ✅ Complete |
| Routing and NAT support | ✅ Complete |
| CLI commands and runtime interaction | ✅ Complete |
| gRPC daemon | ✅ Complete |
| Protobuf/gRPC API | ✅ Complete |
| Structured logging | ✅ Complete |
| Monitoring infrastructure | ✅ Complete |
| eBPF runtime/probe infrastructure | ✅ Complete |
| Process, filesystem, and network eBPF probes | ✅ Complete |
| KVM virtualization components | ✅ Complete |
| Unit test coverage | ⏳ Remaining |
| Integration test suite | ⏳ Remaining |
| End-to-end runtime validation | ⏳ Remaining |
| Testing across real Linux environments | ⏳ Remaining |
| Production hardening | 🔄 Ongoing |

## What Remains

The implementation phase is substantially complete, but a systems project of this kind is not complete without systematic validation.

### 1. Unit Tests

Add focused unit tests throughout the workspace, especially for:

- Image reference parsing
- Registry authentication and manifest handling
- Digest verification
- Layer storage and extraction
- Overlay configuration
- Container state transitions
- cgroup configuration and validation
- Network configuration helpers
- CLI argument and configuration handling
- gRPC request/response conversion
- Monitoring and logging helpers
- eBPF event decoding and shared data structures
- Virtualization configuration and validation

### 2. Integration Tests

Add tests that exercise interactions between crates and Linux subsystems:

- Pull → unpack → create → start → stop → remove
- OverlayFS lifecycle
- cgroup creation and cleanup
- Bridge/veth creation and teardown
- Container-to-host and container-to-network connectivity
- Daemon startup and gRPC client communication
- Persistent container state recovery
- Image-store reuse
- Failure and rollback paths

Privileged integration tests should be isolated and clearly marked so they can be run intentionally on Linux hosts.

### 3. End-to-End Tests

Build a repeatable test environment that validates complete workflows:

```text
OCI registry
    ↓
Image pull
    ↓
Content-addressable storage
    ↓
Layer unpack
    ↓
OverlayFS root
    ↓
Namespaces + cgroups
    ↓
Container process
    ↓
Network namespace
    ↓
Bridge / veth / NAT
    ↓
Lifecycle management
    ↓
Cleanup
```

The same workflows should also be exercised through the daemon/gRPC path.

### 4. Real-Environment Validation

Test Virtualos-rs on multiple Linux distributions and kernel configurations.

Important validation areas include:

- Different Linux kernel versions
- cgroup v2 configurations
- OverlayFS behavior
- Different iproute2/iptables versions
- Root and non-root daemon/client scenarios
- IPv4 and IPv6 networking
- Multiple concurrent containers
- Large image layers
- Container process failures
- Host reboots and stale state cleanup
- eBPF availability and verifier behavior
- KVM availability and VM startup
- Resource-limit enforcement under load

## Possible Improvements

The current implementation provides a strong foundation, but there are many directions in which Virtualos-rs can evolve.

### Runtime and Container Security

- Implement additional Linux namespaces where appropriate, including user namespaces.
- Add Linux capabilities management and capability dropping.
- Add seccomp profiles.
- Support read-only root filesystems and configurable mounts.
- Add device allow/deny policies.
- Improve privilege separation between the daemon and runtime operations.
- Implement safer Unix socket ownership and permission management.
- Add stronger validation of untrusted OCI image metadata.
- Harden cleanup and rollback paths after partial failures.

### OCI and Image Management

- Expand OCI distribution compatibility.
- Support additional authentication mechanisms.
- Add image metadata/config management.
- Implement layer deduplication and garbage collection.
- Add image listing and removal commands.
- Add registry mirrors.
- Add configurable image-store backends.
- Support resumable downloads.
- Improve parallel layer downloads.
- Add image signing and verification.

### Networking

- Improve IPv4/IPv6 dual-stack support.
- Add configurable DNS handling.
- Add port publishing and host-to-container forwarding.
- Add configurable network creation.
- Support multiple isolated networks.
- Replace external networking commands with native netlink APIs where practical.
- Add network namespace lifecycle recovery.
- Improve concurrent network setup and teardown.

### Resource Management

- Expand cgroup v2 support beyond CPU and memory.
- Add process count limits.
- Add I/O controls.
- Add CPU weight and quota configuration.
- Expose resource usage through the API and monitoring layer.
- Improve handling of cgroup cleanup after abnormal termination.

### Observability

- Expand Prometheus metrics coverage.
- Add per-container CPU, memory, I/O, process, and network metrics.
- Add trace correlation between container lifecycle events and eBPF events.
- Add configurable eBPF probe selection.
- Add event buffering and backpressure handling.
- Improve event schemas and versioning.
- Add OpenTelemetry integration.

### Virtualization

- Complete and harden the KVM execution path.
- Add additional virtual devices.
- Improve guest boot configuration.
- Add virtio-based storage and networking.
- Support VM snapshots where practical.
- Add VM lifecycle APIs to the daemon.
- Explore lightweight microVM workflows combining container and VM isolation.

### Reliability and Operations

- Add graceful daemon shutdown and recovery handling.
- Improve crash recovery and stale-resource cleanup.
- Add transactional lifecycle operations where appropriate.
- Add concurrency stress tests.
- Add fuzz testing for parsers and protocol boundaries.
- Add fault-injection testing.
- Add deterministic test fixtures.
- Add CI pipelines for formatting, linting, unit tests, integration tests, and privileged Linux tests.
- Add release automation and reproducible builds.
- Improve configuration management and environment discovery.

## Contributing

Contributions are welcome, particularly in areas that improve correctness and real-world usability.

### Good Areas for Contribution

- Unit and integration tests
- End-to-end test infrastructure
- Linux distribution and kernel compatibility testing
- OCI registry compatibility
- Container networking
- cgroup resource management
- eBPF probes and event processing
- Prometheus metrics
- KVM virtualization
- Security hardening
- Documentation and examples
- Performance benchmarking
- CI/CD and reproducible builds
- Bug reports from real Linux environments

### Suggested Contribution Workflow

1. Fork the repository.
2. Create a focused feature or fix branch.
3. Keep changes scoped to the relevant crate where possible.
4. Add or update tests for behavioral changes.
5. Run formatting and static checks.
6. Build the complete workspace.
7. Test privileged functionality on a disposable Linux environment.
8. Document any kernel, distribution, or privilege requirements.
9. Open a pull request describing the change, validation performed, and any known limitations.

For infrastructure changes, please include the Linux kernel version, distribution, relevant package versions, and the exact workflow used for validation.

## Testing Strategy

The project should ultimately maintain multiple test layers:

```text
                 ┌─────────────────────┐
                 │   Unit Tests        │
                 │ Pure Rust logic     │
                 └──────────┬──────────┘
                            │
                 ┌──────────▼──────────┐
                 │ Integration Tests   │
                 │ Crate + Linux APIs  │
                 └──────────┬──────────┘
                            │
                 ┌──────────▼──────────┐
                 │ End-to-End Tests    │
                 │ Full container flow │
                 └──────────┬──────────┘
                            │
                 ┌──────────▼──────────┐
                 │ Real Environment    │
                 │ Kernel/distribution │
                 └─────────────────────┘
```

The distinction between ordinary unit tests and privileged Linux tests is intentional. Tests requiring namespaces, cgroups, OverlayFS, networking, eBPF, or KVM should run in controlled Linux environments with the required privileges and kernel capabilities.

## Project Philosophy

Virtualos-rs is designed as a systems-programming project for understanding how modern container and virtualization infrastructure is constructed from Linux primitives and Rust.

Rather than treating containers as a black box, the project brings together the underlying mechanisms:

- Linux namespaces for isolation
- cgroup v2 for resource control
- OverlayFS for layered filesystems
- OCI registries and content-addressable storage for images
- Linux bridge/veth networking for connectivity
- gRPC for runtime management
- `tracing` and monitoring for operational visibility
- eBPF for low-level kernel observability
- KVM for hardware-assisted virtualization

The goal is to keep these components understandable, modular, and replaceable while progressively moving toward stronger security, reliability, performance, and production-readiness.

## License

This project is licensed under the [MIT License](LICENSE).
