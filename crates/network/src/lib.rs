use anyhow::{Context, Result};
use std::process::Command;

const BRIDGE_NAME: &str = "docklet0";
const SUBNET: &str = "10.0.0.0/24";
const GATEWAY: &str = "10.0.0.1";

/// Initialise the host bridge and NAT. Run once.
pub fn init_network() -> Result<()> {
    // Create bridge if not exists
    let check = Command::new("ip")
        .args(["link", "show", BRIDGE_NAME])
        .status();
    if check.is_err() || !check.unwrap().success() {
        run_cmd(
            "ip",
            &["link", "add", "name", BRIDGE_NAME, "type", "bridge"],
        )?;
    }

    // Assign IP to bridge (idempotent)
    let _ = run_cmd(
        "ip",
        &[
            "addr",
            "replace",
            &format!("{}/24", GATEWAY),
            "dev",
            BRIDGE_NAME,
        ],
    );
    run_cmd("ip", &["link", "set", BRIDGE_NAME, "up"])?;

    // Enable IP forwarding
    std::fs::write("/proc/sys/net/ipv4/ip_forward", "1")?;

    // iptables NAT rule (idempotent: add only if not present)
    let check_rule = Command::new("iptables")
        .args([
            "-t",
            "nat",
            "-C",
            "POSTROUTING",
            "-s",
            SUBNET,
            "!",
            "-o",
            BRIDGE_NAME,
            "-j",
            "MASQUERADE",
        ])
        .status();
    if check_rule.is_err() || !check_rule.unwrap().success() {
        run_cmd(
            "iptables",
            &[
                "-t",
                "nat",
                "-A",
                "POSTROUTING",
                "-s",
                SUBNET,
                "!",
                "-o",
                BRIDGE_NAME,
                "-j",
                "MASQUERADE",
            ],
        )?;
    }

    Ok(())
}

/// Set up networking for a container.
/// `pid` – child PID, `container_id` used for interface naming, `container_ip` must be inside `SUBNET`.
pub fn setup_container_net(pid: u32, container_id: &str, container_ip: &str) -> Result<()> {
    let host_veth = format!("veth-{}", container_id);
    let container_veth = format!("ceth-{}", container_id);

    // Create veth pair
    run_cmd(
        "ip",
        &[
            "link",
            "add",
            &host_veth,
            "type",
            "veth",
            "peer",
            "name",
            &container_veth,
        ],
    )?;

    // Attach host end to bridge
    run_cmd("ip", &["link", "set", &host_veth, "master", BRIDGE_NAME])?;
    run_cmd("ip", &["link", "set", &host_veth, "up"])?;

    // Move container end into the container's network namespace
    run_cmd(
        "ip",
        &["link", "set", &container_veth, "netns", &pid.to_string()],
    )?;

    // Configure inside the container's namespace via nsenter
    let netns_path = format!("/proc/{}/ns/net", pid);
    run_cmd_ns(&netns_path, "ip", &["link", "set", "lo", "up"])?;
    run_cmd_ns(
        &netns_path,
        "ip",
        &[
            "addr",
            "add",
            &format!("{}/24", container_ip),
            "dev",
            &container_veth,
        ],
    )?;
    run_cmd_ns(&netns_path, "ip", &["link", "set", &container_veth, "up"])?;
    run_cmd_ns(
        &netns_path,
        "ip",
        &[
            "route",
            "add",
            "default",
            "via",
            GATEWAY,
            "dev",
            &container_veth,
        ],
    )?;

    Ok(())
}

/// Remove the host‑side veth after the container stops.
pub fn teardown_container_net(container_id: &str) -> Result<()> {
    let host_veth = format!("veth-{}", container_id);
    // Container end is removed automatically when netns is destroyed.
    let _ = run_cmd("ip", &["link", "delete", &host_veth]);
    Ok(())
}

// ---------- helpers ----------
fn run_cmd(cmd: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run {} {:?}", cmd, args))?;
    if !output.status.success() {
        anyhow::bail!(
            "{} {:?} failed: {}",
            cmd,
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn run_cmd_ns(netns_path: &str, cmd: &str, args: &[&str]) -> Result<()> {
    eprintln!("DEBUG: nsenter --net {} {} {:?}", netns_path, cmd, args);
    if netns_path.is_empty() {
        anyhow::bail!("empty netns path: {netns_path}");
    }
    if !std::path::Path::new(&netns_path).exists() {
        anyhow::bail!("Network namespace file not found: {}", netns_path);
    }
    let output = Command::new("nsenter")
        .arg(format!("--net={}", netns_path))
        .arg(cmd)
        .args(args)
        .output()
        .with_context(|| format!("nsenter {} {:?} failed", cmd, args))?;
    if !output.status.success() {
        anyhow::bail!(
            "nsenter {} {:?} failed: {}",
            cmd,
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
