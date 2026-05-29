//! Build glue: compile the hello-ebpf crate for the BPF target and make its
//! object available to include_bytes_aligned!(OUT_DIR/hello). This is the
//! aya-build approach; a plain `cargo build` then produces one binary.
use std::path::PathBuf;

fn main() {
    let ebpf_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("hello-ebpf");
    // aya-build reads the named package and emits the compiled BPF object.
    aya_build::build_ebpf([aya_build::Toolchain::default()
        .package(ebpf_dir)
        .expect("hello-ebpf crate must exist")])
    .expect("failed to build hello-ebpf");
}
