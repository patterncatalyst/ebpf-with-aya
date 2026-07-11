# 38 · Signal programs: reacting with force

A **signal program**: instead of denying a syscall at an LSM hook, it reacts
to one. A tracepoint on `execve` matches a forbidden filename prefix and
calls `bpf_send_signal(SIGKILL)` to kill the process before the new image
runs. **LAB-ONLY** — the mechanism behind runtime security tooling, and a
fast way to break a real system.

## What it does

- Attaches `#[tracepoint] kill_on_exec` to `syscalls:sys_enter_execve`.
- Reads the filename; if it starts with `/tmp/forbidden`, emits a
  `KillEvent` (pid + comm) on a `RingBuf` and calls
  `bpf_send_signal(SIGKILL)`.
- The loader prints `killed <comm> (pid …)` per event and exports
  `ebpf_signal_kills_total{comm}`.

## Run it

```bash
./demo.sh          # build + deploy to $VM + run a forbidden + a normal binary
./demo.sh build    # just build on the host
```

On the target, `/tmp/forbidden-sleep` is killed on exec (exit 137) while a
normal `sleep` survives; the loader prints each kill.

## Verify on the target

```bash
cp /usr/bin/sleep /tmp/forbidden-sleep
/tmp/forbidden-sleep 60 ; echo "exit=$?"   # -> Killed ; exit=137 (128 + SIGKILL)
sleep 60 &                                  # allowed binary keeps running
```

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the lab
VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches, and runs as
described — `bpf_send_signal(SIGKILL)` from the `sys_enter_execve` tracepoint
kills the forbidden binary before `execve` completes while a normal `sleep`
survives, and the bounded `starts_with` passes the verifier. Attach targets
and struct offsets (such as the filename offset) can be kernel-version-specific.
