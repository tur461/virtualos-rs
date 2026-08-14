use std::{env, path::PathBuf};

fn main() {
    let profile = env::var("PROFILE").unwrap();

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("failed to find workspace root");

    let bpf_object = workspace_root
        .join("target")
        .join("bpfel-unknown-none")
        .join(&profile)
        .join("ebpf_probes");

    println!("cargo:rerun-if-changed={}", bpf_object.display());

    println!(
        "cargo:rustc-env=VIRTUALOS_BPF_OBJECT={}",
        bpf_object.display()
    );
}
