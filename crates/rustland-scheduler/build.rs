fn main() -> anyhow::Result<()> {
    // Generates bpf.rs, bpf_intf.rs/bindings and the libbpf skeleton used by
    // the scx_rustland_core bridge. This intentionally requires the target
    // host's Clang, libbpf, bpftool/BTF and sched_ext-capable kernel headers.
    scx_rustland_core::RustLandBuilder::new()?.build()
}
