//! Headless command-line entrypoint.
//!
//! Reuses the same Rust modules the GUI calls (capture::run,
//! capture::list_interfaces, privilege::get_privilege_status). When the
//! user runs `portfinder` with no subcommand, main.rs launches the GUI
//! instead.

use crate::{capture, privilege, CaptureRequest, CaptureResult, InterfaceInfo};
use clap::{Parser, Subcommand};
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(
    name = "portfinder",
    version,
    about = "Network switch port discovery tool"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Capture one CDP, LLDP, or MNDP packet and print the switch info.
    Capture {
        /// Network interface name (omit to sniff all interfaces).
        #[arg(short, long, default_value = "")]
        interface: String,

        /// Discovery protocol to look for.
        #[arg(
            short,
            long,
            value_parser = ["CDP", "cdp", "LLDP", "lldp", "MNDP", "mndp"],
            default_value = "LLDP",
        )]
        protocol: String,

        /// Output JSON instead of human-readable key/value.
        #[arg(long)]
        json: bool,
    },
    /// List network interfaces visible to libpcap.
    List {
        /// Only show interfaces with a routable IP address.
        #[arg(long)]
        with_ip: bool,

        /// Output JSON instead of a human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Show privilege status (whether packet capture is currently allowed).
    Privileges {
        /// Output JSON instead of human-readable key/value.
        #[arg(long)]
        json: bool,
    },
}

/// Returns process exit code: 0 on success, 1 on error.
pub fn run(cli: Cli) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: failed to start runtime: {e}");
            return 1;
        }
    };

    let result = runtime.block_on(async move {
        match cli.command {
            Commands::Capture {
                interface,
                protocol,
                json,
            } => run_capture(interface, protocol, json).await,
            Commands::List { with_ip, json } => run_list(with_ip, json),
            Commands::Privileges { json } => run_privileges(json),
        }
    });

    // Detach rather than waiting for any in-flight spawn_blocking
    // pcap reads to wind down. The blocking loop in
    // `capture::capture_blocking` polls cancellation every
    // `PCAP_TIMEOUT_MS` (50 ms), so dropping the runtime would
    // normally only stall ~50 ms — but under VM load that has been
    // seen to wedge for seconds, and there's nothing those threads
    // can produce now that the result has been consumed. The
    // process::exit(...) call in main will tear them down anyway.
    runtime.shutdown_background();

    match result {
        Ok(()) => 0,
        Err(msg) => {
            eprintln!("error: {msg}");
            1
        }
    }
}

async fn run_capture(interface: String, protocol: String, json: bool) -> Result<(), String> {
    let cancel = CancellationToken::new();

    // Forward Ctrl+C to the cancellation token so the user can
    // interrupt a long-running capture cleanly. The handler loops
    // and counts presses: the first ^C cancels gracefully, any
    // subsequent ^C aborts via `process::exit(130)` (the standard
    // 128+SIGINT exit code).
    //
    // The loop matters because `tokio::signal::ctrl_c()` installs
    // a SIGINT handler with the OS that *replaces* the kernel
    // default (terminate). Once that handler is in place,
    // signals are absorbed by tokio's stream regardless of whether
    // anyone's awaiting them — so without an explicit second-press
    // escape hatch, a wedged blocking task makes the process
    // unkillable from the terminal short of SIGKILL.
    let cancel_for_signal = cancel.clone();
    tokio::spawn(async move {
        let mut presses = 0u8;
        loop {
            if tokio::signal::ctrl_c().await.is_err() {
                return;
            }
            presses = presses.saturating_add(1);
            if presses == 1 {
                eprintln!("\ninterrupted");
                cancel_for_signal.cancel();
            } else {
                eprintln!("force-exit");
                std::process::exit(130);
            }
        }
    });

    if !json {
        eprintln!(
            "Capturing {} on {}...",
            protocol.to_uppercase(),
            label_for(&interface)
        );
    }

    let request = CaptureRequest {
        interface_name: interface,
        protocol,
    };

    let result = capture::run(request, cancel).await?;
    print_capture_result(&result, json);
    Ok(())
}

fn run_list(with_ip: bool, json: bool) -> Result<(), String> {
    let mut interfaces = capture::list_interfaces()?;
    if with_ip {
        interfaces.retain(|i| i.name.is_empty() || i.has_ip);
    }
    print_interfaces(&interfaces, json);
    Ok(())
}

fn run_privileges(json: bool) -> Result<(), String> {
    let status = privilege::get_privilege_status();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&status).map_err(|e| e.to_string())?
        );
    } else {
        println!("Platform:          {}", status.platform);
        println!("Has access:        {}", yesno(status.has_access));
        println!("Helper installed:  {}", yesno(status.helper_installed));
        println!("In BPF group:      {}", yesno(status.in_bpf_group));
        println!("Can install:       {}", yesno(status.can_install));
        if status.platform == "windows" {
            println!("Npcap installed:   {}", yesno(status.npcap_installed));
            println!("Npcap non-admin:   {}", yesno(status.npcap_non_admin));
        }
    }
    Ok(())
}

fn print_capture_result(r: &CaptureResult, json: bool) {
    if json {
        let _ = serde_json::to_string_pretty(r).map(|s| println!("{s}"));
        return;
    }
    println!("Switch Name:  {}", r.switch_name);
    println!("Switch IP:    {}", r.switch_ip);
    println!("Switch Port:  {}", r.switch_port);
    println!("VLAN:         {}", r.native_vlan);
    println!("Voice VLAN:   {}", r.voice_vlan);
    println!("MTU:          {}", r.mtu);
    println!("Switch Model: {}", r.switch_model);
}

fn print_interfaces(ifaces: &[InterfaceInfo], json: bool) {
    if json {
        let _ = serde_json::to_string_pretty(ifaces).map(|s| println!("{s}"));
        return;
    }
    let name_w = ifaces
        .iter()
        .map(|i| i.name.len())
        .max()
        .unwrap_or(0)
        .max(4);
    println!("{:<width$}  IP   Addresses", "Name", width = name_w);
    for i in ifaces {
        let display_name = if i.name.is_empty() {
            "(all)"
        } else {
            i.name.as_str()
        };
        println!(
            "{:<width$}  {}    {}",
            display_name,
            yesno(i.has_ip),
            i.addresses,
            width = name_w
        );
    }
}

fn label_for(iface: &str) -> &str {
    if iface.is_empty() {
        "all interfaces"
    } else {
        iface
    }
}

fn yesno(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}
