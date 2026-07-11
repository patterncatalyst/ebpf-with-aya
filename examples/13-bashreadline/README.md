# Example 13 — bashreadline (uretprobe on bash's readline)

Capture every command typed at an interactive bash prompt — the classic
first **uprobe** (user-space probe).

## What this shows (new — first user-space probe)

- **uprobes** attach to a function in a *binary or shared library*, not
  the kernel. Here a **uretprobe** fires on the *return* of `readline()`
  in bash.
- `readline()` returns a `char *` to the line the user typed; on return
  we read that pointer — **user memory of the bash process** — into our
  event.
- Attaching by **symbol name + target path**:
  `attach(Some("readline"), 0, "/usr/bin/bash", None)`.

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # build + deploy to VM + run (Ctrl-C to stop)
```

Then, in **another** terminal, open an *interactive* bash on the target
and type:

```bash
ssh -t fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" bash -i
# then type:  echo hello   /   ls -la   /   whoami
```

Each command you type appears in the `PID UID COMMAND` table;
`ebpf_events_total{program="bashreadline"}` climbs in Grafana.

> Only **interactive** prompts call `readline()`. Commands run
> non-interactively (`ssh host 'cmd'`, scripts) won't appear — that's
> expected and is the point: this sees what a human types.

## If nothing appears

`readline` may live in `libreadline` rather than the bash binary on your
distro. Re-run pointing at the library (path on the target):

```bash
# in deploy-to-target's run, or set on the target before running:
READLINE_LIB=/usr/lib64/libreadline.so.8 sudo ./bashreadline
```

Find where the symbol is:

```bash
[vm]$ objdump -T /usr/bin/bash | grep -w readline || nm -D /usr/lib64/libreadline.so.8 | grep -w readline
```

## ⚠ Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the
lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches the
uretprobe, and captures interactive commands as described. Where
`readline` resolves (the `bash` binary vs. `libreadline.so`) is
distro- and build-specific — use the `READLINE_LIB` override and
`objdump`/`nm` if no events appear. Attach targets and struct offsets
can be kernel-version-specific.
