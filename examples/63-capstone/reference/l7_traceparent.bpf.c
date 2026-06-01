/* REFERENCE (canonical, illustrative) — L7 extraction of the W3C traceparent
 * from a socket read, the OBI technique (Chapter 46). Scans the request bytes
 * for "traceparent: 00-<trace_id>-..." and emits an event tagged with the
 * trace_id, joining the eBPF view directly to the app's trace. Sketch: real
 * parsing must bounds-check every access for the verifier. */
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>

struct event { __u8 trace_id[32]; __u32 pid; };
struct { __uint(type, BPF_MAP_TYPE_RINGBUF); __uint(max_entries, 1 << 16); } events SEC(".maps");

/* Attach to the read path (e.g. uprobe on SSL_read for TLS, or a syscall/probe
 * on plaintext read), then within the copied buffer:
 *   - find the "traceparent:" header (case-insensitive)
 *   - skip "00-" version, copy the next 32 hex chars as trace_id
 *   - reserve an `event`, fill trace_id + pid, submit
 * The user-space loader then tags ebpf_capstone_* with that trace_id, so the
 * kernel metrics line up with the exact span in Tempo. */

char _license[] SEC("license") = "GPL";
