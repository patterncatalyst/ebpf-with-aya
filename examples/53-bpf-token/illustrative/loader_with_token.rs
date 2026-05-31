// ILLUSTRATIVE — where a BPF token plugs into an Aya loader. Aya's loader-side
// token support is EMERGING; this shows the intended shape, not a settled API.
// With libbpf the equivalent is a single `bpf_token_path` in the open options.
//
// The key idea: the PROGRAM is unchanged. The token is a property of HOW it is
// loaded, supplied by the loader — so the same Aya program from any earlier
// chapter can load inside a delegated, unprivileged container.

use aya::EbpfLoader;

fn main() -> anyhow::Result<()> {
    // A privileged runtime mounted a delegated bpffs into our container here,
    // e.g. with: delegate_cmds=prog_load:map_create, delegate_progs=socket_filter,
    // delegate_maps=ringbuf. We have no init-namespace CAP_BPF of our own.
    let delegated_bpffs = "/sys/fs/bpf";

    let _ebpf = EbpfLoader::new()
        // EMERGING (shape, not a guaranteed API): derive a token from the
        // delegated bpffs and thread its fd through prog-load / map-create /
        // link-create. Until Aya exposes this, libbpf-based loaders do it via
        // bpf_token_path; track the Aya release notes.
        //
        // .token_path(delegated_bpffs)
        .load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/your-program")))?;

    let _ = delegated_bpffs;
    // ... then the same load()/attach()/poll() as any other chapter.
    Ok(())
}
