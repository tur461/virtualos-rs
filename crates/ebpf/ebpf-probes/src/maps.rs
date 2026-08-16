use aya_ebpf::{
    macros::map,
    maps::{PerCpuArray, RingBuf},
};

use crate::{
    events::{
        ExecEvent, ExitEvent, ForkEvent, FsCloseEvent, FsOpenEvent, FsRenameEvent, FsUnlinkEvent,
        NetSocketEvent,
    },
    networking::types::SocketEvent,
};

#[map]
pub static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[map]
pub static FS_OPEN_SCRATCH: PerCpuArray<FsOpenEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static FS_CLOSE_SCRATCH: PerCpuArray<FsCloseEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static FS_UNLINK_SCRATCH: PerCpuArray<FsUnlinkEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static FS_RENAME_SCRATCH: PerCpuArray<FsRenameEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static NW_ACCEPT_SCRATCH: PerCpuArray<NetSocketEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static NW_BIND_SCRATCH: PerCpuArray<NetSocketEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static NW_CONNECT_SCRATCH: PerCpuArray<NetSocketEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static NW_SOCKET_SCRATCH: PerCpuArray<SocketEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static NW_UDP_SCRATCH: PerCpuArray<SocketEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static PROC_FORK_SCRATCH: PerCpuArray<ForkEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static PROC_EXIT_SCRATCH: PerCpuArray<ExitEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static PROC_EXEC_SCRATCH: PerCpuArray<ExecEvent> = PerCpuArray::with_max_entries(1, 0);
