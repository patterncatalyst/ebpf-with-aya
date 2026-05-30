---
title: "uprobe + bashreadline"
order: 13
part: User-space & language probing
description: The first user-space probe — attach a uretprobe to bash's readline() to capture commands typed at interactive prompts, learning how uprobes target a binary or library by symbol and read the calling process's memory.
duration: 25 minutes
---

Everything in *Tracing the kernel* attached to the kernel. This part
turns the lens around: **uprobes** attach to functions inside
user-space programs and libraries. The classic first example —
`bashreadline` — attaches to bash's `readline()` and shows every
command a user types at an interactive prompt. It's a small program
that teaches the whole uprobe model.

The code is in `examples/13-bashreadline/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="uprobe-menu"
   alt="User-space probing surfaces: an eBPF uprobe can attach to an executable symbol, a shared-library symbol, or a USDT marker; an entry probe sees arguments and a return probe sees results."
   caption="Figure 13.1 — the user-space probing menu" %}

## What a uprobe is

A **uprobe** attaches to a function in a *binary or shared library* —
identified by a file path plus a symbol name (or a raw offset). When
any process executes that code, your program runs. A **uretprobe** is
the same idea at function *return*, where the return value is
available. The kernel implements them, but they fire on *user-space*
code, and the memory they read belongs to the traced **process**, not
the kernel.

That last point is the crucial shift. In Chapters 7–8 we read kernel
memory; in Chapter 9 we read user memory that happened to be a syscall
argument. A uprobe lives entirely in user-space territory: arguments,
return values, and any pointers they carry are all in the traced
process's address space, read with the **user** probe helpers.

## Why readline

When you type a command at an interactive bash prompt and press Enter,
bash got that line from `readline()` — the GNU Readline function that
handles line editing, history, and completion. Its return value is a
`char *` pointing to the line you typed. So a uretprobe on `readline`
captures interactive shell commands at the moment of entry, before any
parsing — a beautifully direct demonstration, and a genuinely useful
audit primitive.

It only sees **interactive** input. A command run non-interactively
(`ssh host 'cmd'`, a shell script) never calls `readline`. That's not a
limitation to work around — it's the precise semantics: this sees what
a human typed.

## The kernel side

A uretprobe whose entire job is to read the return pointer:

```rust
#[uretprobe]
pub fn readline_ret(ctx: RetProbeContext) -> u32 {
    let line_ptr: *const u8 = match ctx.ret() { Some(p) => p, None => return 0 };
    if let Some(mut slot) = EVENTS.reserve::<ReadlineEvent>(0) {
        let ev = slot.as_mut_ptr();
        unsafe {
            (*ev).pid  = (bpf_get_current_pid_tgid() >> 32) as u32;
            (*ev).uid  = (bpf_get_current_uid_gid() & 0xffff_ffff) as u32;
            (*ev).comm = bpf_get_current_comm().unwrap_or([0u8; 16]);
            let _ = bpf_probe_read_user_str_bytes(line_ptr, &mut (*ev).line);
        }
        slot.submit(0);
    }
    0
}
```

`ctx.ret()` gives the return value — the uretprobe analogue of
`ctx.arg(n)` — which for `readline` is a `char *` to the line the user
just typed. That pointer is into the **bash process's** memory, so we
can't dereference it; `bpf_probe_read_user_str_bytes` copies the string
into the event. Note the shape we'll reuse for every ring-buffer
program: `reserve` a typed slot, write *through* its pointer (filling the
cheap-to-get `pid`/`uid`/`comm` directly, then the user string), and
`submit`. There's no entry probe and no map to bridge — the line only
exists at *return*, so a single uretprobe is the whole program.

## The user side

Attaching a uprobe needs a **symbol** and a **target file**:

```rust
let prog: &mut UProbe = ebpf.program_mut("readline_ret").unwrap().try_into()?;
prog.load()?;
prog.attach(Some("readline"), 0, "/usr/bin/bash", None)?;
```

The arguments are: the symbol name (`Some("readline")`), an offset
(`0`, meaning the symbol's start), the target path, and an optional PID
to restrict to (`None` = every process that runs this code). Aya
resolves the symbol to a file offset for you by reading the binary's
symbol table.

One real-world wrinkle: **where `readline` lives varies.** On some
builds it's compiled into the `bash` binary; on others it's in
`libreadline.so`. If no events appear, the symbol is in the library —
point the target at it instead (the example reads a `READLINE_LIB`
override). You find out with:

```bash
[vm]$ objdump -T /usr/bin/bash | grep -w readline || nm -D /usr/lib64/libreadline.so.8 | grep -w readline
```

This is the uprobe tax: you attach to a *concrete artifact on disk*, so
you have to know which artifact holds the symbol. `bpftool`/`objdump`/
`nm` are how you find out.

## Build, deploy, observe

```bash
cd examples/13-bashreadline && ./demo.sh
```

Once it's attached, open an **interactive** bash on the target in
another terminal and type a few commands:

```bash
ssh -t fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" bash -i
# then type:  echo hello   /   ls -la   /   whoami
```

Each line appears in the `PID UID COMMAND` table the instant you press
Enter, and `ebpf_events_total{program="bashreadline"}` climbs in
Grafana.

## Cross-check

```bash
[vm]$ sudo bashreadline-bpfcc
```

The BCC tool does exactly this. Type in one shell, watch both tools
print the same lines.

## What you learned

- uprobes/uretprobes attach to user-space functions by **path +
  symbol**; uretprobes read the return value via `ctx.ret()`.
- The memory you read belongs to the **traced process** — user probe
  helpers, always.
- You attach to a concrete on-disk artifact, so you must know which
  binary or library holds the symbol (`objdump`/`nm`).

Next: pointing a uprobe at a **Rust** binary, where symbol mangling and
calling conventions come into play.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm the `UProbe`/`#[uretprobe]`/`RetProbeContext::ret()` API, the
`attach` signature, and where `readline` resolves on Fedora 44's bash.
The first build and run are the test.*
