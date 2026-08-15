use anyhow::{Context, Result};
use aya::{
    Ebpf, include_bytes_aligned,
    maps::{RingBuf, ring_buf::RingBufItem},
    programs::{CgroupAttachMode, SockOps, TracePoint},
};
use aya_log::EbpfLogger;
use std::fs::File;
use tokio::sync::mpsc;
use tracing::info;

const BPF_OBJECT: &[u8] = include_bytes_aligned!(env!("VIRTUALOS_BPF_OBJECT"));

const EVENT_TYPE_SOCKET: u16 = 1;

const EVENT_HEADER_SIZE: usize = 4;

// SocketEvent:
//
// timestamp_ns  u64 = 8
// pid           u32 = 4
// tgid          u32 = 4
// uid           u32 = 4
// gid           u32 = 4
// socket_cookie u64 = 8
// family        u8  = 1
// protocol      u8  = 1
// kind          u8  = 1
// old_state     u8  = 1
// new_state     u8  = 1
// padding       alignment
// local_port    u16
// remote_port   u16
// local_addr    [u8; 16]
// remote_addr   [u8; 16]
// bytes         u64
//
// With repr(C), the exact size is 80 bytes.
const SOCKET_EVENT_SIZE: usize = 80;

pub struct EbpfManager {
    bpf: Ebpf,
}

impl EbpfManager {
    pub fn load() -> Result<Self> {
        let mut bpf = Ebpf::load(BPF_OBJECT).context("Failed to load BPF object")?;

        if let Err(err) = EbpfLogger::init(&mut bpf) {
            tracing::warn!("Failed to initialize eBPF logger: {err:#}");
        }

        Ok(Self { bpf })
    }

    /// Start the common RingBuf event consumer.
    ///
    /// Unlike the previous implementation, this does NOT move the
    /// Ebpf object out of EbpfManager. This allows us to attach
    /// additional programs afterwards.
    pub fn start_event_reader(&mut self) -> Result<mpsc::Receiver<BpfEvent>> {
        let (tx, rx) = mpsc::channel(1024);

        let mut bpf = std::mem::replace(
            &mut self.bpf,
            Ebpf::load(&[]).context("Failed to create placeholder BPF")?,
        );

        std::thread::spawn(move || {
            let map = match bpf.map_mut("EVENTS") {
                Some(map) => map,
                None => return,
            };
            let mut ring_buf = match RingBuf::try_from(map) {
                Ok(buf) => buf,
                Err(_) => return,
            };

            loop {
                match ring_buf.next() {
                    Some(item) => {
                        if let Some(event) = parse_event(item)
                            && tx.blocking_send(event).is_err()
                        {
                            break;
                        }
                    }

                    None => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                }
            }
        });

        info!("eBPF RingBuf event reader started");

        Ok(rx)
    }

    /// Attach process tracing.
    pub fn start_exec_tracing(&mut self) -> Result<()> {
        attach_tracepoint(
            &mut self.bpf,
            "trace_execve",
            "syscalls",
            "sys_enter_execve",
        )?;

        info!("eBPF execve tracing attached");

        Ok(())
    }

    /// Attach filesystem tracepoints.
    pub fn start_filesystem_tracing(&mut self) -> Result<()> {
        attach_tracepoint(&mut self.bpf, "trace_close", "syscalls", "sys_enter_close")?;

        attach_tracepoint(
            &mut self.bpf,
            "trace_unlink",
            "syscalls",
            "sys_enter_unlink",
        )?;

        attach_tracepoint(
            &mut self.bpf,
            "trace_unlinkat",
            "syscalls",
            "sys_enter_unlinkat",
        )?;

        attach_tracepoint(
            &mut self.bpf,
            "trace_rename",
            "syscalls",
            "sys_enter_rename",
        )?;

        attach_tracepoint(
            &mut self.bpf,
            "trace_renameat",
            "syscalls",
            "sys_enter_renameat",
        )?;

        attach_tracepoint(
            &mut self.bpf,
            "trace_renameat2",
            "syscalls",
            "sys_enter_renameat2",
        )?;

        info!("eBPF filesystem tracing attached");

        Ok(())
    }

    /// Attach traditional networking syscall tracepoints.
    pub fn start_network_syscall_tracing(&mut self) -> Result<()> {
        attach_tracepoint(
            &mut self.bpf,
            "trace_connect",
            "syscalls",
            "sys_enter_connect",
        )?;

        attach_tracepoint(
            &mut self.bpf,
            "trace_accept",
            "syscalls",
            "sys_enter_accept",
        )?;

        attach_tracepoint(&mut self.bpf, "trace_bind", "syscalls", "sys_enter_bind")?;

        info!("eBPF network syscall tracing attached");

        Ok(())
    }

    /// Attach TCP sockops program.
    ///
    /// The program must be attached to an appropriate cgroup from
    /// the daemon/loader side. SockOps is not a tracepoint.
    pub fn load_tcp_sockops(&mut self) -> Result<()> {
        let program: &mut SockOps = self
            .bpf
            .program_mut("tcp_socket_ops")
            .context("Program 'tcp_socket_ops' not found")?
            .try_into()?;

        program.load()?;

        info!("eBPF TCP sockops program loaded");

        Ok(())
    }

    /// Attach TCP sockops to a cgroup.
    pub fn attach_tcp_sockops(&mut self, cgroup_path: &std::path::Path) -> Result<()> {
        let program: &mut SockOps = self
            .bpf
            .program_mut("tcp_socket_ops")
            .context("Program 'tcp_socket_ops' not found")?
            .try_into()?;

        let fd = File::open(cgroup_path)?;
        program.attach(fd, CgroupAttachMode::Single)?;

        info!(
            cgroup = %cgroup_path.display(),
            "eBPF TCP sockops attached"
        );

        Ok(())
    }

    /// Attach all currently implemented probes.
    pub fn start_all(&mut self) -> Result<mpsc::Receiver<BpfEvent>> {
        self.start_exec_tracing()?;

        self.start_filesystem_tracing()?;

        self.start_network_syscall_tracing()?;

        let rx = self.start_event_reader()?;

        info!("All eBPF tracepoints attached");

        Ok(rx)
    }
}

fn attach_tracepoint(
    bpf: &mut Ebpf,
    program_name: &str,
    category: &str,
    tracepoint: &str,
) -> Result<()> {
    let program: &mut TracePoint = bpf
        .program_mut(program_name)
        .with_context(|| format!("eBPF program '{}' not found", program_name))?
        .try_into()
        .with_context(|| format!("eBPF program '{}' is not a TracePoint", program_name))?;

    program
        .load()
        .with_context(|| format!("Failed to load eBPF program '{}'", program_name))?;

    program.attach(category, tracepoint).with_context(|| {
        format!(
            "Failed to attach '{}' to {}/{}",
            program_name, category, tracepoint
        )
    })?;

    Ok(())
}

// -----------------------------------------------------------------------------
// Events
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub enum BpfEvent {
    Socket(SocketEvent),
    Unknown { event_type: u16, data: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct SocketEvent {
    pub timestamp_ns: u64,

    pub pid: u32,
    pub tgid: u32,
    pub uid: u32,
    pub gid: u32,

    pub socket_cookie: u64,

    pub family: AddressFamily,
    pub protocol: TransportProtocol,
    pub kind: SocketEventKind,

    pub old_state: u8,
    pub new_state: u8,

    pub local_port: u16,
    pub remote_port: u16,

    pub local_addr: [u8; 16],
    pub remote_addr: [u8; 16],

    pub bytes: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum AddressFamily {
    Unknown,
    IPv4,
    IPv6,
}

impl AddressFamily {
    fn from_raw(value: u8) -> Self {
        match value {
            4 => Self::IPv4,
            6 => Self::IPv6,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TransportProtocol {
    Unknown,
    Tcp,
    Udp,
}

impl TransportProtocol {
    fn from_raw(value: u8) -> Self {
        match value {
            6 => Self::Tcp,
            17 => Self::Udp,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SocketEventKind {
    Unknown,

    TcpConnect,
    TcpPassiveEstablished,
    TcpListen,
    TcpStateChange,
    TcpClose,

    UdpSend,
    UdpReceive,
    UdpBind,
    UdpConnect,
}

impl SocketEventKind {
    fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::TcpConnect,
            2 => Self::TcpPassiveEstablished,
            3 => Self::TcpListen,
            4 => Self::TcpStateChange,
            5 => Self::TcpClose,

            10 => Self::UdpSend,
            11 => Self::UdpReceive,
            12 => Self::UdpBind,
            13 => Self::UdpConnect,

            _ => Self::Unknown,
        }
    }
}

// -----------------------------------------------------------------------------
// Event parser
// -----------------------------------------------------------------------------

fn parse_event(item: RingBufItem<'_>) -> Option<BpfEvent> {
    let data: &[u8] = &item;

    if data.len() < EVENT_HEADER_SIZE {
        tracing::warn!(size = data.len(), "Received truncated eBPF event");

        return None;
    }

    let event_type = u16::from_ne_bytes([data[0], data[1]]);

    let event_size = u16::from_ne_bytes([data[2], data[3]]) as usize;

    if event_size > data.len() - EVENT_HEADER_SIZE {
        tracing::warn!(
            event_size,
            available = data.len(),
            "Invalid eBPF event size"
        );

        return None;
    }

    match event_type {
        EVENT_TYPE_SOCKET => {
            parse_socket_event(&data[EVENT_HEADER_SIZE..], event_size).map(BpfEvent::Socket)
        }

        _ => Some(BpfEvent::Unknown {
            event_type,
            data: data[EVENT_HEADER_SIZE..].to_vec(),
        }),
    }
}

fn parse_socket_event(data: &[u8], event_size: usize) -> Option<SocketEvent> {
    if event_size < SOCKET_EVENT_SIZE {
        tracing::warn!(
            event_size,
            expected = SOCKET_EVENT_SIZE,
            "Truncated SocketEvent"
        );

        return None;
    }

    /*
     * Keep the offsets explicit.
     *
     * Kernel:
     *
     * #[repr(C)]
     * struct SocketEvent {
     *     timestamp_ns: u64,
     *     pid: u32,
     *     tgid: u32,
     *     uid: u32,
     *     gid: u32,
     *     socket_cookie: u64,
     *     family: u8,
     *     protocol: u8,
     *     kind: u8,
     *     old_state: u8,
     *     new_state: u8,
     *     ...
     * }
     *
     * IMPORTANT:
     * These offsets must match the actual BPF-side repr(C)
     * layout. See note below about generating the ABI.
     */

    let timestamp_ns = read_u64(data, 0)?;
    let pid = read_u32(data, 8)?;
    let tgid = read_u32(data, 12)?;
    let uid = read_u32(data, 16)?;
    let gid = read_u32(data, 20)?;

    let socket_cookie = read_u64(data, 24)?;

    let family = AddressFamily::from_raw(*data.get(32)?);
    let protocol = TransportProtocol::from_raw(*data.get(33)?);
    let kind = SocketEventKind::from_raw(*data.get(34)?);

    let old_state = *data.get(35)?;
    let new_state = *data.get(36)?;

    /*
     * repr(C) alignment puts u16 at offset 38.
     */
    let local_port = read_u16(data, 38)?;
    let remote_port = read_u16(data, 40)?;

    let mut local_addr = [0u8; 16];
    local_addr.copy_from_slice(data.get(42..58)?);

    let mut remote_addr = [0u8; 16];
    remote_addr.copy_from_slice(data.get(58..74)?);

    /*
     * u64 alignment.
     *
     * 74 -> padding -> 80
     */
    let bytes = read_u64(data, 80)?;

    Some(SocketEvent {
        timestamp_ns,

        pid,
        tgid,
        uid,
        gid,

        socket_cookie,

        family,
        protocol,
        kind,

        old_state,
        new_state,

        local_port,
        remote_port,

        local_addr,
        remote_addr,

        bytes,
    })
}

#[inline]
fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset + 2)?;

    Some(u16::from_ne_bytes([bytes[0], bytes[1]]))
}

#[inline]
fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;

    Some(u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[inline]
fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    let bytes = data.get(offset..offset + 8)?;

    Some(u64::from_ne_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}
