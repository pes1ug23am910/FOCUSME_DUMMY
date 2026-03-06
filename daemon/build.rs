// ============================================================
// FILE:        build.rs
// MODULE:      Daemon build script
// TASK:        T-010
// PLATFORM:    cross (build-time platform detection)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, daemon build configuration
// DEPENDENCIES: None (build script)
// TEST COVERAGE: N/A (build-time only)
// KNOWN LIMITATIONS: Windows service metadata requires rc file
// ============================================================

fn main() {
    // Windows-specific: embed application manifest and version info
    #[cfg(target_os = "windows")]
    {
        // Embed Windows resource file for service metadata
        // TODO: Create daemon/resources/focusme.rc with version info
        // winres::WindowsResource::new()
        //     .set_icon("resources/focusme.ico")
        //     .set("ProductName", "FocusMe Daemon")
        //     .set("FileDescription", "FocusMe Enforcement Daemon Service")
        //     .set("LegalCopyright", "Copyright © 2025 FocusMe")
        //     .compile()
        //     .expect("Failed to compile Windows resources");

        println!("cargo:rerun-if-changed=resources/focusme.rc");
    }

    // Linux-specific: build eBPF programs if clang is available
    #[cfg(target_os = "linux")]
    {
        // TODO: Integrate libbpf-cargo for eBPF skeleton generation
        // libbpf_cargo::SkeletonBuilder::new()
        //     .source("../linux/bpf/focusme_lsm.bpf.c")
        //     .build_and_generate("src/focusme_lsm.skel.rs")
        //     .expect("Failed to build eBPF skeleton");

        println!("cargo:rerun-if-changed=../linux/bpf/focusme_lsm.bpf.c");
    }

    // All platforms: embed build metadata
    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rustc-env=BUILD_TIMESTAMP={}",
        chrono_build_timestamp()
    );
}

fn chrono_build_timestamp() -> String {
    // Cross-platform timestamp without external commands
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Simple UTC timestamp calculation
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let s = time_secs % 60;
    // Approximate date from days since epoch (good enough for build stamps)
    let mut y: u64 = 1970;
    let mut remaining = days;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if remaining < days_in_year { break; }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days: &[u64] = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
        &[31,29,31,30,31,30,31,31,30,31,30,31]
    } else {
        &[31,28,31,30,31,30,31,31,30,31,30,31]
    };
    let mut m: u64 = 1;
    for &md in month_days {
        if remaining < md { break; }
        remaining -= md;
        m += 1;
    }
    let d = remaining + 1;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, hours, mins, s)
}
