---
title: "Detaching and pinning: outliving the loader"
order: 48
part: Advanced kernel surface
description: "Open the advanced part with the lifecycle question every earlier chapter dodged: what happens to a program when its loader exits? Learn how BPF objects are reference-counted, pin a program, map, and link to bpffs so they survive the loader, read the pinned map from a separate process, and detach by removing the pin."
duration: 40 minutes
---

Every loader so far has ended the same way: it attaches a program, loops while
holding a file descriptor, and when you press Ctrl-C the program detaches and
vanishes. That's the right shape for a tool you run interactively, but it
hides a question that matters the moment you operate eBPF for real: **what
keeps a program alive, and how do you make it outlive the process that loaded
it?** The answer — reference counting and **pinning to bpffs** — is the
foundation the rest of this part and the operating chapters build on, so we
start here.

The code is in `examples/48-pin-demo/`. `./demo.sh` there pins a program and
exits, reads it back from a fresh process, and detaches; its `README.md` has
the details.

{% include excalidraw.html
   file="pinning"
   alt="A loader loads a tracepoint program, an exec-count map, and a link (attachment), then pins all three to /sys/fs/bpf (bpffs), where the pins hold references, and the loader exits. Because the pins hold references, the program, map, and link outlive the loader until the pin is removed. Any reader process can open the pinned map to read results; removing the pin (rm) drops the reference and detaches."
   caption="Figure 48.1 — Pins in bpffs hold references, so objects outlive the loader" %}

## Reference counting, briefly

Every BPF object — a program, a map, a **link** (the handle representing one
attachment) — lives as long as something holds a reference to it. The usual
reference is a **file descriptor** in your loader. When the loader exits, the
kernel closes its fds, the reference counts drop to zero, and the objects are
freed: the program detaches and the map's data is gone. That's why Ctrl-C ends
the show.

To keep an object alive past the loader, something *else* must hold a
reference. The standard mechanism is the **BPF filesystem** (`bpffs`, mounted
at `/sys/fs/bpf`). Pinning an object creates a file there that holds a
reference of its own:

```bash
[vm]$ mount | grep bpf            # bpffs is usually already mounted at /sys/fs/bpf
[vm]$ ls /sys/fs/bpf              # pins show up here as files
```

A pin is just a named reference. While the pin exists the object stays alive,
even with no process holding it; remove the pin and the reference drops.

## Pinning with Aya

Three things are worth pinning, and Aya pins each:

- **The map**, so its data survives and other processes can open it.
- **The link**, so the *attachment* survives — without this the program would
  detach even if the program object itself were pinned.
- **The program** (optional once the link is pinned, but handy for
  inspection).

```rust
// after load() + attach()
let link_id = prog.attach("syscalls", "sys_enter_execve")?;
let link = prog.take_link(link_id)?;          // take ownership of the attachment
let fd_link: FdLink = link.try_into()?;        // convert to a pinnable fd-backed link
fd_link.pin("/sys/fs/bpf/ebpf-aya/execs_link")?;   // attachment now outlives us

let map: MapData = /* the EXECS map */;
map.pin("/sys/fs/bpf/ebpf-aya/EXECS")?;        // data now outlives us
// loader exits here — the program keeps counting execs
```

Reading the key moves:

- **`take_link`** hands you the owned link; converting it to an **`FdLink`**
  gives a handle backed by a file descriptor, which is the kind Aya can pin.
  `pin` writes the bpffs entry that holds the attachment open.
- **`MapData::pin`** does the same for the map. The directory must already
  exist on bpffs (`mkdir /sys/fs/bpf/ebpf-aya`), since pinning won't create
  parent directories.
- Once both are pinned, the loader can drop everything and exit. The program
  stays attached and keeps writing to the map.

### Reading a pinned map from elsewhere

Because the map is a named object on bpffs, *any* process with access can open
it — the loader doesn't need to be running:

```rust
let map = MapData::from_pin("/sys/fs/bpf/ebpf-aya/EXECS")?;
let execs: HashMap<_, u32, u64> = HashMap::try_from(Map::HashMap(map))?;
println!("execs so far: {}", execs.get(&0, 0)?);
```

This decoupling is the whole point: one process installs and pins, a different
process (or many, over time) reads — the lifecycle of the eBPF program is no
longer tied to any single program. That is exactly how zero-downtime managers
operate, a thread Part 9 picks up.

## Build, deploy, observe

```bash
cd examples/48-pin-demo && ./demo.sh
```

The demo runs `pinctl load` (load, attach, pin link + map, **exit**), then
shows the pins in `/sys/fs/bpf` with the loader gone, runs `pinctl read` from a
fresh process a couple of times so you can watch the exec counter climb with no
loader running, and finally `pinctl detach` to remove the pins.

**In the terminal** you'll see the count rise between reads — proof the program
kept working after its loader exited. **In Grafana** (`127.0.0.1:3000`), the
`read` step exports `ebpf_pinned_execs_total`, so you can graph the persisted
counter that any reader can pick up.

## Cross-check

```bash
[vm]$ sudo bpftool prog show                 # the tracepoint program, still loaded
[vm]$ sudo bpftool link show                 # the pinned link holding the attachment
[vm]$ sudo bpftool map show                  # the EXECS map
[vm]$ sudo bpftool map dump pinned /sys/fs/bpf/ebpf-aya/EXECS   # its contents
[vm]$ ls -l /sys/fs/bpf/ebpf-aya/            # the pins themselves
```

`bpftool prog show` listing the program while no loader runs is the proof that
the pin holds it; `bpftool map dump pinned` reads the same counter your `read`
command does. Removing the pins (`pinctl detach` or `rm`) and re-running
`bpftool prog show` shows it gone — the reference is what kept it alive.

## What you learned

- BPF programs, maps, and links are **reference-counted**; when the loader's
  file descriptors close, the objects are freed and the program detaches.
- **Pinning to bpffs** (`/sys/fs/bpf`) creates a named reference that keeps an
  object alive past the loader — pin the **link** to keep the attachment, the
  **map** to keep (and share) the data.
- A pinned map can be opened by **any** process, decoupling the program's
  lifecycle from its loader; **removing the pin** drops the reference and
  detaches.

Next, Chapter 49 builds on persistence with programs that act on syscalls
directly, beginning the tour of the modern BPF surface.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that `/sys/fs/bpf` is mounted; the Aya
pinning API used here (`take_link`, `FdLink::try_from`, `FdLink::pin`,
`MapData::pin`, `MapData::from_pin`); that the program keeps counting after the
loader exits; that `bpftool prog/link/map show` and `map dump pinned` see the
pinned objects; and that removing the pin detaches the program.*
