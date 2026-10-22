#!/usr/bin/env python3
"""Official Home Assistant Core Python's real StateMachine hot in-memory read.
No network, auth, services, recorder, SQLite, integrations, or devices.
Benchmarks a single narrowly matched cache-read workload, NOT parity.
"""
import asyncio
import json
from pathlib import Path
import tempfile
import time

import homeassistant.core

COUNT=1232
WARMUP=5
ROUNDS=30

def proc_memory_kib(key):
    try:
        for line in Path("/proc/self/smaps_rollup").read_text().splitlines():
            name,_,rest=line.partition(":")
            if name==key:
                return int(rest.strip().split()[0])
    except (OSError, ValueError, IndexError):
        pass
    return 0

async def measure():
    with tempfile.TemporaryDirectory(prefix="p14-core-") as directory:
        hass=homeassistant.core.HomeAssistant(directory)
        for i in range(COUNT):
            hass.states.async_set(f"sensor.sensor_{i}",str(i))
        names=[f"sensor.sensor_{i}" for i in range(COUNT)]
        expected=[str(i) for i in range(COUNT)]
        assert len(hass.states.async_all())==COUNT
        round_ns=[]
        for round_id in range(WARMUP+ROUNDS):
            checksum=0
            begin=time.perf_counter_ns()
            for i,name in enumerate(names):
                state=hass.states.get(name)
                assert state is not None and state.state==expected[i]
                checksum+=i
            elapsed=time.perf_counter_ns()-begin
            assert checksum==(COUNT-1)*COUNT//2
            if round_id>=WARMUP:
                round_ns.append(elapsed)
        print(json.dumps({
            "schema_version":1,
            "backend":"official_core_python",
            "scope":"hot_in_memory_state_read",
            "entities":COUNT,
            "warmup_rounds":WARMUP,
            "round_ns":round_ns,
            "rss_kb":proc_memory_kib("Rss"),
            "pss_kb":proc_memory_kib("Pss"),
            "durable":False,
            "api":"HomeAssistant.states.get(entity_id), synthetic State.state string",
            "note":"Single process hot-state retrieval only, not a full Home Assistant workload",
        }, sort_keys=True))
if __name__=="__main__":
    asyncio.run(measure())
