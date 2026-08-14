use aya_ebpf::{
    macros::map,
    maps::{PerCpuArray, RingBuf},
};

use crate::events::{FsOpenEvent, FsRenameEvent, FsUnlinkEvent};

#[map]
pub static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[map]
pub static FS_OPEN_SCRATCH: PerCpuArray<FsOpenEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static FS_UNLINK_SCRATCH: PerCpuArray<FsUnlinkEvent> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static FS_RENAME_SCRATCH: PerCpuArray<FsRenameEvent> = PerCpuArray::with_max_entries(1, 0);
