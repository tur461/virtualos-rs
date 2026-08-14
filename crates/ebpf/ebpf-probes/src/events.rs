#![allow(dead_code)]

use core::mem::size_of;

use crate::{
    maps::EVENTS,
    //networking::SocketEvent
};

pub const EVENT_EXEC: u32 = 1;
pub const EVENT_FORK: u32 = 2;
pub const EVENT_EXIT: u32 = 3;

pub const EVENT_FS_OPEN: u32 = 10;
pub const EVENT_FS_CLOSE: u32 = 11;
pub const EVENT_FS_UNLINK: u32 = 12;
pub const EVENT_FS_RENAME: u32 = 13;

pub const EVENT_NET_CONNECT: u32 = 20;
pub const EVENT_NET_ACCEPT: u32 = 21;
pub const EVENT_NET_BIND: u32 = 22;
pub const EVENT_NET_CLOSE: u32 = 23;

pub const TASK_COMM_LEN: usize = 16;
pub const FILENAME_LEN: usize = 256;
pub const FS_PATH_LEN: usize = 256;
pub const NET_COMM_LEN: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExecEvent {
    pub event_type: u32,

    pub pid: u32,
    pub tgid: u32,

    pub uid: u32,
    pub gid: u32,

    pub cgroup_id: u64,

    pub comm: [u8; TASK_COMM_LEN],
    pub filename: [u8; FILENAME_LEN],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForkEvent {
    pub event_type: u32,

    // Parent process
    pub parent_pid: u32,
    pub parent_tgid: u32,

    // Newly created process
    pub child_pid: u32,
    pub child_tgid: u32,

    pub uid: u32,
    pub gid: u32,

    pub cgroup_id: u64,

    pub comm: [u8; TASK_COMM_LEN],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExitEvent {
    pub event_type: u32,

    pub pid: u32,
    pub tgid: u32,

    pub exit_code: i32,

    pub uid: u32,
    pub gid: u32,

    pub cgroup_id: u64,

    pub comm: [u8; TASK_COMM_LEN],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FsOpenEvent {
    pub event_type: u32,

    pub pid: u32,
    pub tgid: u32,

    pub uid: u32,
    pub gid: u32,

    pub cgroup_id: u64,

    pub flags: u64,

    pub comm: [u8; TASK_COMM_LEN],

    pub path: [u8; FS_PATH_LEN],

    pub path_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FsCloseEvent {
    pub event_type: u32,

    pub pid: u32,
    pub tgid: u32,

    pub uid: u32,
    pub gid: u32,

    pub cgroup_id: u64,

    pub fd: i64,

    pub comm: [u8; TASK_COMM_LEN],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FsUnlinkEvent {
    pub event_type: u32,

    pub pid: u32,
    pub tgid: u32,

    pub uid: u32,
    pub gid: u32,

    pub cgroup_id: u64,

    pub flags: u64,

    pub comm: [u8; TASK_COMM_LEN],

    pub path: [u8; FS_PATH_LEN],
    pub path_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FsRenameEvent {
    pub event_type: u32,

    pub pid: u32,
    pub tgid: u32,

    pub uid: u32,
    pub gid: u32,

    pub cgroup_id: u64,

    pub flags: u64,

    pub comm: [u8; TASK_COMM_LEN],

    pub old_path: [u8; FS_PATH_LEN],
    pub old_path_len: u32,

    pub new_path: [u8; FS_PATH_LEN],
    pub new_path_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NetSocketEvent {
    pub event_type: u32,

    pub pid: u32,
    pub tgid: u32,

    pub uid: u32,
    pub gid: u32,

    pub cgroup_id: u64,

    pub fd: i32,

    pub family: u16,
    pub socket_type: u16,
    pub protocol: u16,

    pub _pad: u16,

    pub comm: [u8; NET_COMM_LEN],
}

#[repr(u16)]
#[derive(Clone, Copy)]
pub enum EventType {
    Unknown = 0,
    Socket = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventHeader {
    pub event_type: EventType,
    pub size: u16,
}

// #[repr(C)]
// #[derive(Clone, Copy)]
// pub struct NetworkEvent {
//     pub header: EventHeader,
//     pub socket: SocketEvent,
// }
//
// impl NetworkEvent {
//     #[inline(always)]
//     pub fn socket(event: SocketEvent) -> Self {
//         Self {
//             header: EventHeader {
//                 event_type: EventType::Socket,
//                 size: core::mem::size_of::<SocketEvent>() as u16,
//             },
//             socket: event,
//         }
//     }
// }

const _: () = {
    assert!(size_of::<ForkEvent>() <= 128);
};

// #[inline(always)]
// pub fn emit(event: NetworkEvent) -> Result<(), i64> {
//     let Some(mut entry) = EVENTS.reserve::<NetworkEvent>(0) else {
//         return Err(-1);
//     };
//
//     entry.write(event);
//
//     entry.submit(0);
//
//     Ok(())
// }
