fn main() {
    // On Windows, the pcap crate links against wpcap.lib, which makes the
    // resulting binary refuse to even start when wpcap.dll (Npcap) isn't
    // installed — the user gets a "code execution cannot proceed because
    // wpcap.dll was not found" dialog before the app's privilege-warning
    // UI ever has a chance to render.
    //
    // Marking wpcap.dll as a delay-loaded import lets the OS skip the
    // resolution at process start; the DLL is only loaded when the first
    // pcap function is actually called. The app launches normally, our
    // get_privilege_status() command reports npcapInstalled=false, and
    // the existing UI banner shows the Npcap download link.
    //
    // delayimp.lib provides the runtime shim that performs the lazy load.
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg-bin=portfinder=/DELAYLOAD:wpcap.dll");
        println!("cargo:rustc-link-arg-bin=portfinder=delayimp.lib");
    }

    tauri_build::build()
}
