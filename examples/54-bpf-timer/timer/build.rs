use std::path::PathBuf;
fn main() {
    let ebpf_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("timer-ebpf");
    aya_build::build_ebpf([aya_build::Toolchain::default()
        .package(ebpf_dir).expect("timer-ebpf crate must exist")])
    .expect("failed to build timer-ebpf");
}
