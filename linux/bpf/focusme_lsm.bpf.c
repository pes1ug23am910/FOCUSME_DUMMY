// ============================================================
// FILE:        focusme_lsm.bpf.c
// MODULE:      Layer 1 — Enforcement Engine > Linux eBPF LSM
// TASK:        T-016
// PLATFORM:    linux
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, Linux eBPF exec blocking
// DEPENDENCIES: libbpf, CO-RE (BTF), Linux 5.7+ with CONFIG_BPF_LSM=y
// TEST COVERAGE: IT-03 (execve returns EPERM for blocked path)
// KNOWN LIMITATIONS: Requires CONFIG_BPF_LSM=y (validate with T-001).
//                    Cannot block kernel threads or init process.
//                    [BLOCKED T-001] Kernel config validation needed.
// ANTI-CIRCUMVENTION: eBPF LSM hooks cannot be detached without CAP_SYS_ADMIN.
//                     Pinned to /sys/fs/bpf/ to persist across daemon restarts.
//                     Operates on host namespace — not bypassable via mount namespace.
// ============================================================

// SPDX-License-Identifier: GPL-2.0
// Required for eBPF programs attached to LSM hooks

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

// Maximum number of blocked paths
#define MAX_BLOCKED_PATHS 256
// Maximum path length
#define MAX_PATH_LEN 256

// Map: blocked executable paths (set semantics — value is unused)
// Key: path string (null-terminated, MAX_PATH_LEN)
// Populated by userspace loader (loader.rs)
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_BLOCKED_PATHS);
    __type(key, char[MAX_PATH_LEN]);
    __type(value, __u8);
} blocked_paths SEC(".maps");

// Map: audit log ring buffer for blocked events
// Userspace reads from this to log block events
struct blocked_event {
    __u32 pid;
    __u32 uid;
    char path[MAX_PATH_LEN];
};

struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 1 << 16); // 64KB ring buffer
} events SEC(".maps");

// LSM hook: bprm_check_security
// Called before a program is executed via execve/execveat
// Return 0 to allow, -EPERM to deny
SEC("lsm/bprm_check_security")
int BPF_PROG(focusme_exec_block, struct linux_binprm *bprm)
{
    char path_buf[MAX_PATH_LEN] = {};
    struct file *file;
    struct dentry *dentry;
    __u8 *blocked;

    // Read the executable file from bprm
    file = BPF_CORE_READ(bprm, file);
    if (!file)
        return 0; // Allow if we can't read

    // Read the path from the dentry
    dentry = BPF_CORE_READ(file, f_path.dentry);
    if (!dentry)
        return 0;

    // Read the filename
    // NOTE: This gets the last component only. For full path matching,
    // use d_path helper or iterate the dentry chain.
    const char *name = BPF_CORE_READ(dentry, d_name.name);
    bpf_probe_read_kernel_str(path_buf, sizeof(path_buf), name);

    // Look up in blocked paths map
    blocked = bpf_map_lookup_elem(&blocked_paths, path_buf);
    if (blocked) {
        // Log the blocked event
        struct blocked_event *evt;
        evt = bpf_ringbuf_reserve(&events, sizeof(*evt), 0);
        if (evt) {
            evt->pid = bpf_get_current_pid_tgid() >> 32;
            evt->uid = bpf_get_current_uid_gid() & 0xFFFFFFFF;
            __builtin_memcpy(evt->path, path_buf, sizeof(path_buf));
            bpf_ringbuf_submit(evt, 0);
        }

        // DENY execution
        return -1; // -EPERM
    }

    // ALLOW execution
    return 0;
}

// License declaration (required for LSM attachment)
char LICENSE[] SEC("license") = "GPL";
