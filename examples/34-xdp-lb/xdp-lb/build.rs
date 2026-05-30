use std::path::PathBuf;
fn main() {
    let ebpf_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("xdp-lb-ebpf");
    aya_build::build_ebpf([aya_build::Toolchain::default()
        .package(ebpf_dir).expect("xdp-lb-ebpf crate must exist")])
    .expect("failed to build xdp-lb-ebpf");
}
