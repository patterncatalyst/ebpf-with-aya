use std::path::PathBuf;
fn main() {
    let ebpf_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("opensnoop-ebpf");
    aya_build::build_ebpf([aya_build::Toolchain::default()
        .package(ebpf_dir)
        .expect("opensnoop-ebpf crate must exist")])
    .expect("failed to build opensnoop-ebpf");
}
