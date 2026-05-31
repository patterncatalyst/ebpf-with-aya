---
title: "BPF LSM: from observing to deciding"
order: 37
part: Security & LSM
description: "The shift from watching the kernel to mediating it: attach a BPF program to an LSM hook whose return value allows (0) or denies (-EPERM) an operation, and confine a single container's outbound connections by cgroup without touching the host."
duration: 45 minutes
---

Every program in this book so far has *reported*: it watched an event and
wrote a number or a record. An LSM program is different in kind — its
**return value is a decision the kernel obeys**. Linux Security Modules are
the mediation layer behind SELinux and AppArmor: hundreds of `security_*`
hook points where the kernel asks "is this allowed?" before doing
something. **BPF LSM** (kernel 5.7+) lets you answer that question with an
eBPF program — return `0` to allow, a negative errno to deny. This part is
where eBPF stops describing the kernel and starts governing it, and this
chapter is the model plus a first, contained policy: confine one
container's outbound connections.

The code is in `examples/37-lsm-confine/`. `./demo.sh` there builds,
deploys, and runs it; its `README.md` covers what it does and how to drive
it.

> **Before this part — enable the BPF LSM.** BPF LSM only works if the
> `bpf` LSM is active in the kernel's LSM list. On most Fedora 44 installs
> it already is; check on the target with:
>
> ```bash
> [vm]$ cat /sys/kernel/security/lsm        # the list must include "bpf"
> ```
>
> If `bpf` is missing, add it and reboot — `scripts/lab/enable-bpf-lsm.sh
> ebpf-target` does exactly that (it appends `bpf` to the `lsm=` kernel
> cmdline with `grubby` and reboots). The demo runs this for you.

{% include excalidraw.html
   file="lsm-decide"
   alt="A process calls connect(), which reaches the LSM hook security_socket_connect. The attached BPF LSM program decides: if the caller's cgroup is OK it returns 0 (allow) and the connect proceeds; if the caller is in a confined cgroup it returns -EPERM (deny) and the connect is blocked. The program's return value decides; here the decision is scoped by cgroup id, confining one container while leaving the host untouched."
   caption="Figure 37.1 — The LSM program's return value decides: 0 allows, negative denies" %}

## The mediation model

An LSM hook fires *inside* a syscall, after argument checks but before the
action commits. The kernel calls every attached LSM in turn; if any returns
non-zero, the operation fails with that errno. A BPF LSM program is just
another voice in that chain:

- **Return `0`** — this program allows the operation (others may still deny).
- **Return a negative errno** (`-EPERM`, `-EACCES`, …) — deny, and the
  syscall returns that error to user space.
- **Respect prior denials.** The kernel passes the running return value as
  the hook's last argument; a well-behaved program checks it and, if a
  previous LSM already denied, returns that value unchanged rather than
  overriding it with a `0`.

This is **Mandatory Access Control**: the policy is enforced by the kernel,
not the application, and the application can't opt out. The contrast with
the rest of the book is sharp — a kprobe that returns a value changes
nothing; an LSM program's return value is the whole point.

## How the code works

Our policy: a process whose **cgroup** is on a confined list may not open
outbound connections. Everything else is allowed. Scoping by cgroup is what
makes this safe and useful — you confine *one container* and the host keeps
working.

### The program

```rust
#[map] static CONFINED: HashMap<u64, u8> = HashMap::with_max_entries(64, 0);
#[map] static DENIED:   HashMap<u32, u64> = HashMap::with_max_entries(1, 0);

#[lsm(hook = "socket_connect")]
pub fn confine_connect(ctx: LsmContext) -> i32 {
    // socket_connect args: (sock, address, addrlen, ret). The last is the
    // running verdict — if a prior LSM already denied, don't override it.
    let prior: i32 = unsafe { ctx.arg(3) };
    if prior != 0 { return prior; }

    let cgid = unsafe { bpf_get_current_cgroup_id() };
    if unsafe { CONFINED.get(&cgid) }.is_some() {
        bump(&DENIED, 0, 1);
        return -1; // -EPERM
    }
    0 // allow
}
```

Walking it:

- **`#[lsm(hook = "socket_connect")]`** attaches to the
  `security_socket_connect` hook — the kernel's "may this socket connect?"
  question. The program type carries BTF type information, which is why it
  can be attached to a named kernel hook at all.
- **`bpf_get_current_cgroup_id()`** is a plain helper — no struct walking.
  It returns the cgroup id of the calling task, the same 64-bit id you'd
  match a container by (Chapter 16). We don't even look at the destination
  address; the policy here is "this cgroup gets no outbound connections."
- **The decision** is the return: `-1` (`-EPERM`) denies, `0` allows. We
  bump a `DENIED` counter on each block so user space can report how often
  the policy fired.
- **`prior`** is the running verdict the kernel threads through; returning
  it when it's already non-zero keeps us a good citizen in the LSM chain.

### Loading an LSM program (BTF required)

LSM programs attach by resolving the hook against the kernel's BTF, so the
loader reads BTF and passes it at load time:

```rust
let btf = Btf::from_sys_fs()?;                       // /sys/kernel/btf/vmlinux
let prog: &mut Lsm = ebpf.program_mut("confine_connect").unwrap().try_into()?;
prog.load("socket_connect", &btf)?;                  // resolve + verify
prog.attach()?;                                      // now mediating
```

Before attaching, the loader fills `CONFINED` with the cgroup id to confine.
A cgroup-v2 directory's id is its inode number, so the loader `stat`s the
cgroup path and inserts that:

```rust
let cg_path = std::env::var("CONFINE_CGROUP").unwrap_or("/sys/fs/cgroup/confined".into());
let cgid = std::fs::metadata(&cg_path)?.ino();       // cgroup-v2 id = dir inode
let mut confined: HashMap<_, u64, u8> = HashMap::try_from(ebpf.map_mut("CONFINED").unwrap())?;
confined.insert(cgid, 1, 0)?;
```

Then it drains `DENIED` on a timer into `ebpf_lsm_denied_total`.

## Build, deploy, observe

```bash
cd examples/37-lsm-confine && ./demo.sh
```

The demo ensures the BPF LSM is enabled, creates a cgroup
`/sys/fs/cgroup/confined`, confines it, and then runs two `curl`s in a loop:
one from inside the confined cgroup and one from a normal shell. The
confined `curl` fails with a connection error (the kernel returned `-EPERM`
from `connect`), the normal one succeeds, and `ebpf_lsm_denied_total` climbs
in Grafana. Stop the loader and both succeed again.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `rate(ebpf_lsm_denied_total[1m])` — blocked connection attempts per second.

## Cross-check

On the target (`[vm]$` — `ssh fedora@$(scripts/lab/vm-ip.sh ebpf-target)`):

```bash
[vm]$ cat /sys/kernel/security/lsm                  # must include "bpf"
[vm]$ sudo bpftool prog show | grep lsm             # the lsm prog is loaded
[vm]$ stat -c '%i %n' /sys/fs/cgroup/confined       # the cgroup id we confined
# run a connect from inside the confined cgroup and watch it fail:
[vm]$ sudo bash -c 'echo $$ > /sys/fs/cgroup/confined/cgroup.procs; curl -m2 http://example.com'
curl: (7) Couldn't connect to server
```

The confined `curl` failing with a connect error while the same command in
an unconfined shell succeeds is the policy working; `bpftool prog show`
listing an `lsm` program confirms what's enforcing it.

## What you learned

- **BPF LSM** turns an eBPF program into an access-control decision: attach
  to a `security_*` hook, return `0` to allow or a negative errno to deny,
  and respect the prior verdict in the chain.
- LSM programs **load against kernel BTF** (`Btf::from_sys_fs()` →
  `Lsm::load(hook, &btf)` → `attach()`), and the `bpf` LSM must be active in
  the kernel's LSM list.
- **cgroup-scoped policy**: `bpf_get_current_cgroup_id()` lets you confine
  one container while leaving the host alone — MAC you can target.

Next, Chapter 38 takes the other enforcement style — instead of denying a
syscall, *react* to one with a **signal program** that kills an offending
process on the spot.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run (kernel ≥ 5.7, `bpf` in the LSM list): the
Aya LSM API (`#[lsm(hook=…)]`, `Lsm::load(hook, &btf)`, `attach()`), the
`LsmContext` argument indexing including the trailing return value at index
3 for `socket_connect`, that returning `-1` actually fails `connect` with
`EPERM`, and that a cgroup-v2 directory's inode equals the id from
`bpf_get_current_cgroup_id()` (may need `name_to_handle_at` instead of
`stat`).*
