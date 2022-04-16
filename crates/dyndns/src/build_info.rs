pub fn print_build_info() {
    println!(
        "
Build date:          {}
Build timestamp:     {}
Build version:       {}
Commit SHA:          {:?}
Commit timestamp:    {:?}
Commit branch:       {:?}
Commit SemVer:       {:?}
Rust channel:        {}
Rust commit date:    {}
Rust commit SHA:     {}
Rust host triple:    {}
Rust llvm version:   {}
Rust version:        {}
Cargo target triple: {}
Cargo profile:       {}
Cargo features:      {}
Host platform:       {}
Host OS:             {}
Host memory:         {}
Host CPU:            {}
Host CPU core count: {}
Host CPU brand:      {}
",
        env!("VERGEN_BUILD_DATE"),
        env!("VERGEN_BUILD_TIMESTAMP"),
        env!("VERGEN_BUILD_SEMVER"),
        option_env!("VERGEN_GIT_SHA"),
        option_env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        option_env!("VERGEN_GIT_BRANCH"),
        option_env!("VERGEN_GIT_SEMVER_LIGHTWEIGHT"),
        env!("VERGEN_RUSTC_CHANNEL"),
        env!("VERGEN_RUSTC_COMMIT_DATE"),
        env!("VERGEN_RUSTC_COMMIT_HASH"),
        env!("VERGEN_RUSTC_HOST_TRIPLE"),
        env!("VERGEN_RUSTC_LLVM_VERSION"),
        env!("VERGEN_RUSTC_SEMVER"),
        env!("VERGEN_CARGO_TARGET_TRIPLE"),
        env!("VERGEN_CARGO_PROFILE"),
        env!("VERGEN_CARGO_FEATURES"),
        env!("VERGEN_SYSINFO_NAME"),
        env!("VERGEN_SYSINFO_OS_VERSION"),
        env!("VERGEN_SYSINFO_TOTAL_MEMORY"),
        env!("VERGEN_SYSINFO_CPU_VENDOR"),
        env!("VERGEN_SYSINFO_CPU_CORE_COUNT"),
        env!("VERGEN_SYSINFO_CPU_BRAND"),
    );
}
