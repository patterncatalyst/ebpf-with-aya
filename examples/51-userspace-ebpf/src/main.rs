//! userspace-ebpf — run real eBPF bytecode in a user-space VM (rbpf), with no
//! kernel, no root, and no lab VM. Assembles a tiny program, registers a Rust
//! helper the bytecode calls, and runs it both interpreted and JIT-compiled.
//!
//! UNVERIFIED: confirm the rbpf API (assembler::assemble, EbpfVmRaw::new,
//! register_helper, execute_program, jit_compile/execute_program_jit) against
//! the current crate version. JIT is x86-64 only.
use anyhow::{anyhow, Result};

// A helper the eBPF program can call (helper key 1). In the kernel this slot
// would be something like bpf_map_lookup_elem; here it's just a Rust function.
fn double(arg: u64, _: u64, _: u64, _: u64, _: u64) -> u64 {
    arg * 2
}

// eBPF assembly: load the first byte of the memory buffer, call helper 1, return.
const ASM: &str = "
ldxb r1, [r1+0]
call 1
exit
";

fn main() -> Result<()> {
    let prog = rbpf::assembler::assemble(ASM).map_err(|e| anyhow!("assemble: {e}"))?;

    if std::env::args().any(|a| a == "--disasm") {
        println!("disassembly of the assembled eBPF bytecode:");
        for insn in rbpf::disassembler::to_insn_vec(&prog) {
            println!("  {}", insn.desc);
        }
        return Ok(());
    }

    let mut mem = [21u8, 0, 0, 0];
    let mut vm = rbpf::EbpfVmRaw::new(Some(&prog)).map_err(|e| anyhow!("vm: {e}"))?;
    vm.register_helper(1, double).map_err(|e| anyhow!("register_helper: {e}"))?;

    // interpreter
    let interp = vm.execute_program(&mut mem).map_err(|e| anyhow!("interp: {e}"))?;

    // JIT (x86-64 only); fall back to the interpreter result elsewhere
    #[cfg(target_arch = "x86_64")]
    let jit = {
        vm.jit_compile().map_err(|e| anyhow!("jit_compile: {e}"))?;
        unsafe { vm.execute_program_jit(&mut mem) }.map_err(|e| anyhow!("jit run: {e}"))?
    };
    #[cfg(not(target_arch = "x86_64"))]
    let jit = interp; // JIT compiler is x86-64 only

    println!("interpreter={interp} jit={jit}  (mem[0]=21, helper doubles -> expect 42)");
    if interp != jit {
        return Err(anyhow!("interpreter and JIT disagree: {interp} vs {jit}"));
    }
    Ok(())
}
