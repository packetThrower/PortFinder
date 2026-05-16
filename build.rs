// Build script.
//
// Two platform-specific knobs, both no-ops on the platforms they
// don't apply to:
//
//   1. Windows: embed resources/icons/icon.ico into the PE
//      resource section so Explorer / Taskbar / Alt-Tab / Start
//      menu render the PortFinder icon on PortFinder.exe.
//      cargo-packager's `icons` config covers the installer-level
//      branding; this is a separate layer the PE itself carries.
//
//   2. Windows: mark wpcap.dll (Npcap) as a delay-loaded import.
//      The `pcap` crate links against wpcap.lib, which makes the
//      resulting binary refuse to start when wpcap.dll isn't on
//      disk — the user gets a "code execution cannot proceed
//      because wpcap.dll was not found" dialog before the app's
//      privilege-warning banner ever has a chance to render.
//      Delay-loading lets the OS skip the resolution at process
//      start; the DLL is only loaded when the first pcap function
//      is actually called. The privilege module then reports
//      `npcapInstalled=false` and the in-app banner shows the
//      Npcap download link.
//
// `embed_resource::compile` is a no-op on non-Windows targets, so
// this builds cleanly on macOS / Linux too.

fn main() {
    #[cfg(target_os = "windows")]
    {
        // Bin name is the capitalised "PortFinder" set in
        // [[bin]] name = "PortFinder"; the linker arg targets that
        // specific binary so debug-built test binaries (which
        // don't link wpcap) don't pick up the flag.
        println!("cargo:rustc-link-arg-bin=PortFinder=/DELAYLOAD:wpcap.dll");
        println!("cargo:rustc-link-arg-bin=PortFinder=delayimp.lib");
    }

    // `compile` returns a status type that's only useful on
    // Windows for failure diagnostics. We bias toward "the build
    // succeeded if the binary linked" — if rc.exe / windres
    // genuinely failed we'd see a link-time error downstream.
    let _ = embed_resource::compile("resources/icon.rc", embed_resource::NONE);
}
