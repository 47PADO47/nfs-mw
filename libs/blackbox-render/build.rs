fn main() {
    println!("cargo:rerun-if-env-changed=STREAMLINE_SDK");
    #[cfg(feature = "streamline")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let sdk = std::env::var("STREAMLINE_SDK")
            .expect("streamline requires STREAMLINE_SDK pointing to NVIDIA Streamline SDK 2.14.16");
        println!("cargo:rerun-if-changed=src/gpu/dlss/bridge.cpp");
        let mut build = cc::Build::new();
        build.cpp(true).define("_WIN32_WINNT", "0x0A00").define("NTDDI_VERSION", "0x0A000000");
        // The bridge uses only the C runtime and SDK POD types; no C++ runtime DLL is needed.
        build.cpp_link_stdlib(None);
        build
            .cpp(true)
            .std("c++17")
            .include(format!("{sdk}/include"))
            .file("src/gpu/dlss/bridge.cpp")
            .compile("blackbox_streamline");
        println!("cargo:rustc-link-lib=wintrust");
        println!("cargo:rustc-link-lib=crypt32");
    }
}
