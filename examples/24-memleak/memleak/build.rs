use std::path::PathBuf;

fn main() {
    let ebpf_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("memleak-ebpf");
    // aya-build 0.1.3: build the named package for the BPF target and embed it.
    let ebpf_dir = ebpf_dir.to_str().expect("ebpf dir path is valid UTF-8");
    aya_build::build_ebpf(
        [aya_build::Package {
            name: "memleak-ebpf",
            root_dir: ebpf_dir,
            no_default_features: false,
            features: &[],
        }],
        aya_build::Toolchain::default(),
    )
    .expect("failed to build memleak-ebpf");
}
