#!/usr/bin/env python3.14
"""Minimal FastAPI target to observe. One JSON endpoint plus a /work endpoint
that does measurable CPU + file I/O so eBPF probes have something to see."""
import os
from fastapi import FastAPI

app = FastAPI(title="ebpf-fastapi-target")

@app.get("/")
def root():
    return {"service": "fastapi-target", "pid": os.getpid()}

@app.get("/work")
def work(n: int = 1000):
    # a little CPU + a file open, so opensnoop/execsnoop-style probes see activity
    total = sum(i * i for i in range(n))
    with open("/etc/hostname") as f:
        host = f.read().strip()
    return {"sum": total, "host": host}
