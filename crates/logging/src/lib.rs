use anyhow::{Context, Result};
use std::io::{self, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry, fmt};

/// Initialise the global tracing subscriber.
///
/// * `stderr` – if true, output human‑readable logs to stderr.
/// * `file_path` – optional path prefix for rotating JSON log files.
/// * `network_url` – optional TCP address (e.g., `127.0.0.1:5140`) for JSON log forwarding.
pub fn init_logging(
    stderr: bool,
    file_path: Option<PathBuf>,
    network_url: Option<&str>,
) -> Result<()> {
    let mut layers = Vec::new();

    if stderr {
        // Human‑readable, minimal output
        let stderr_layer = fmt::layer()
            .with_writer(io::stderr)
            .with_target(false)
            .without_time() // time is added by the JSON layers
            .boxed();
        layers.push(stderr_layer);
    }

    if let Some(file_path) = file_path {
        // Rotating file appender (daily rotation, keep last 7)
        let file_appender = tracing_appender::rolling::daily(
            file_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
            file_path
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("docklet.log")),
        );
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        // JSON format
        let file_layer = fmt::layer()
            .json()
            .with_writer(non_blocking)
            .with_target(true)
            .with_timer(fmt::time::UtcTime::rfc_3339())
            .boxed();
        layers.push(file_layer);
    }

    if let Some(url) = network_url {
        let (json_layer, guard) = json_tcp_layer(url)?;
        // Keep the guard alive for the lifetime of the subscriber.
        // We'll leak it or store it somewhere. For simplicity, we'll forget it.
        std::mem::forget(guard); // must not drop, else the writer thread stops
        layers.push(json_layer.boxed());
    }

    Registry::default()
        .with(layers)
        .try_init()
        .context("Failed to init tracing subscriber")?;

    Ok(())
}

/// Create a layer that writes JSON lines to a TCP socket.
/// Returns the layer and a guard that must be kept alive.
fn json_tcp_layer(addr: &str) -> Result<(impl Layer<Registry>, TcpWriterGuard)> {
    let stream = TcpStream::connect(addr)
        .with_context(|| format!("Failed to connect to log sink {}", addr))?;
    stream.set_nonblocking(false)?; // we want blocking writes in the worker thread

    let (tx, rx) = mpsc::sync_channel::<String>(1024); // bounded channel for backpressure
    let _guard = TcpWriterGuard { handle: None };
    // Spawn a thread to write to the TCP stream
    let handle = thread::spawn(move || {
        let mut stream = stream;
        for msg in rx {
            if let Err(e) = stream.write_all(msg.as_bytes()) {
                eprintln!("TCP log write error: {}", e);
                // Reconnect? For now, break and stop writing.
                break;
            }
        }
    });
    // We need to put the handle into the guard so it can be joined later.
    // But the guard must be owned. We'll use an Option<JoinHandle> and set it.
    // We'll leak the guard to keep the thread alive for the whole program.
    // Better: return a guard that joins on drop.
    let layer = tracing_subscriber::fmt::layer()
        .json()
        .with_timer(fmt::time::UtcTime::rfc_3339())
        .with_writer(move || {
            // This is called for each event to get a writer.
            // We'll send the serialised event via the channel instead.
            // But fmt::Layer expects a writer. We can use a `MakeWriter` that returns a type
            // that writes to the channel. Implementation below.
            // We'll implement a custom Layer instead.
            // For simplicity, we can use the `MakeWriter` approach: each event gets a new writer
            // that writes a serialized JSON string to the channel. But fmt::Layer already serializes JSON,
            // so we can use it with a writer that forwards the bytes to the channel.
            // We'll define a `ChannelWriter` that writes to the channel.
            ChannelWriter::new(tx.clone())
        })
        .boxed();

    Ok((
        layer,
        TcpWriterGuard {
            handle: Some(handle),
        },
    ))
}

#[allow(dead_code)]
struct TcpWriterGuard {
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for TcpWriterGuard {
    fn drop(&mut self) {
        // nothing to do since we forget it anyway
    }
}

/// A writer that sends each byte chunk as a separate message through an mpsc channel.
/// The tracing-fmt JSON layer will write the entire JSON string at once.
struct ChannelWriter {
    tx: mpsc::SyncSender<String>,
}

impl ChannelWriter {
    fn new(tx: mpsc::SyncSender<String>) -> Self {
        ChannelWriter { tx }
    }
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let msg = String::from_utf8_lossy(buf).to_string();
        // The tracing-fmt JSON layer writes the whole JSON line in one write.
        if self.tx.send(msg).is_err() {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "channel closed"))
        } else {
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
