use anyhow::{Context, Result};
use aya::{
    Ebpf, include_bytes_aligned,
    maps::{RingBuf, ring_buf::RingBufItem},
    programs::TracePoint,
};
use aya_log::EbpfLogger;
use tokio::sync::mpsc;
use tracing::info;

const BPF_OBJECT: &[u8] = include_bytes_aligned!(env!("VIRTUALOS_BPF_OBJECT"));

pub struct EbpfManager {
    bpf: Ebpf,
}

impl EbpfManager {
    pub fn load() -> Result<Self> {
        let mut bpf = Ebpf::load(BPF_OBJECT).context("Failed to load BPF object")?;

        let _ = EbpfLogger::init(&mut bpf).context("Failed to init BPF logger")?;

        Ok(Self { bpf })
    }

    pub async fn start_exec_tracing(&mut self) -> Result<mpsc::Receiver<ExecEvent>> {
        let program: &mut TracePoint = self
            .bpf
            .program_mut("trace_execve")
            .context("Program trace_execve not found")?
            .try_into()?;

        program.load()?;

        program
            .attach("syscalls", "sys_enter_execve")
            .context("Failed to attach tracepoint")?;

        info!("eBPF execve tracing attached");

        let (tx, rx) = mpsc::channel(1024);

        // Move the BPF object out of self so the worker owns it.
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
                Ok(ring_buf) => ring_buf,
                Err(_) => return,
            };

            loop {
                match ring_buf.next() {
                    Some(item) => {
                        let event = parse_event(item);

                        if tx.blocking_send(event).is_err() {
                            break;
                        }
                    }

                    None => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
        });
        Ok(rx)
    }
}

#[derive(Debug, Clone)]
pub struct ExecEvent {
    pub pid: u32,
    pub filename: String,
}

fn parse_event(item: RingBufItem<'_>) -> ExecEvent {
    let data = &item;

    if data.len() < 260 {
        return ExecEvent {
            pid: 0,
            filename: String::new(),
        };
    }

    let pid = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]);

    let filename_bytes = &data[4..260];

    let filename = String::from_utf8_lossy(filename_bytes)
        .trim_end_matches('\0')
        .to_string();

    ExecEvent { pid, filename }
}
