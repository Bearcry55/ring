use clap::Parser;
use colored::*;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, ToSocketAddrs};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::time::timeout;
use futures::future;
use surge_ping::{Client, Config, IcmpPacket, PingIdentifier, PingSequence};

/// RING: Rust Internet Network Grapher — Multi-host + Multi-port TCP scanner with ICMP ping
#[derive(Parser, Debug)]
#[command(
author,
version,
about = "RING: Rust Internet Network Grapher (TCP ping + ICMP ping + multi-host + multi-port)",
          long_about = "RING is a fast, parallel network connectivity scanner that supports both TCP port checking and ICMP ping.

          EXAMPLES:
          ring google.com                           # Basic TCP check on port 80
          ring google.com -p 80,443,22              # Check multiple ports
          ring google.com cloudflare.com --ping     # Multiple hosts with ICMP ping
          ring 192.168.1.1-10 -p 22,80,443 --json  # Scan IP range with JSON output
          ring example.com -p 1000-2000 --once      # Port range scan, run once
          ring api.example.com --ping -t 5000 -c 5  # Custom timeout and attempt count"
)]
struct Args {
    /// One or more hostnames or IPs
    hosts: Vec<String>,

    /// Ports to connect to, comma-separated or range (e.g. 80,443,1000-1005)
    #[arg(short, long, default_value = "80")]
    ports: String,

    /// Number of attempts per host+port
    #[arg(short, long, default_value_t = 3)]
    count: u32,

    /// Timeout for each connection attempt in milliseconds (default: 2000)
    #[arg(short = 't', long, default_value_t = 2000)]
    timeout: u64,

    /// Quiet mode (suppress individual result lines)
    #[arg(short, long)]
    quiet: bool,

    /// Output results in JSON format
    #[arg(short, long)]
    json: bool,

    /// Run once instead of continuously
    #[arg(short = 'i', long)]
    once: bool,

    /// Enable ICMP ping in addition to TCP checks
    #[arg(long)]
    ping: bool,

    /// ICMP ping timeout in milliseconds (default: 1000)
    #[arg(long, default_value_t = 1000)]
    ping_timeout: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct HostResult {
    host: String,
    port: Option<u16>,
    test_type: String, // "tcp" or "icmp"
    attempts: u32,
    successful: u32,
    success_rate: f64,
    avg_response_time_ms: Option<f64>,
    response_times: Vec<u128>,
    status: String, // "up", "down", "partial"
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ScanResult {
    scan_timestamp: String,
    results: Vec<HostResult>,
}

fn parse_ports(s: &str) -> Vec<u16> {
    s.split(',')
    .flat_map(|part| {
        if part.contains('-') {
            let mut range = part.split('-');
            if let (Some(start), Some(end)) = (range.next(), range.next()) {
                if let (Ok(start), Ok(end)) = (start.parse(), end.parse()) {
                    return (start..=end).collect::<Vec<u16>>();
                }
            }
            vec![]
        } else {
            part.parse().ok().into_iter().collect()
        }
    })
    .collect()
}

async fn tcp_check(host: String, port: u16, count: u32, timeout_ms: u64) -> HostResult {
    let mut response_times = Vec::new();
    let mut successful = 0;
    let mut last_error = None;

    let addr = format!("{}:{}", host, port);
    let resolved = addr.to_socket_addrs().ok().and_then(|mut iter| iter.next());

    if resolved.is_none() {
        return HostResult {
            host: host.to_string(),
            port: Some(port),
            test_type: "tcp".to_string(),
            attempts: count,
            successful: 0,
            success_rate: 0.0,
            avg_response_time_ms: None,
            response_times: vec![],
            status: "down".to_string(),
            error: Some("dns_resolution_failed".to_string()),
        };
    }

    let socket_addr = resolved.unwrap();
    let timeout_dur = Duration::from_millis(timeout_ms);

    for _ in 1..=count {
        let start = Instant::now();
        let result = timeout(timeout_dur, TcpStream::connect(socket_addr)).await;
        let elapsed = start.elapsed();

        match result {
            Ok(Ok(_)) => {
                successful += 1;
                response_times.push(elapsed.as_millis());
            }
            Ok(Err(e)) => {
                last_error = Some(format!("connection_error: {}", e));
            }
            Err(_) => {
                last_error = Some("timeout".to_string());
            }
        }
    }

    let success_rate = successful as f64 / count as f64;
    let avg_response_time = if !response_times.is_empty() {
        Some(response_times.iter().sum::<u128>() as f64 / response_times.len() as f64)
    } else {
        None
    };

    let status = match success_rate {
        1.0 => "up",
        0.0 => "down",
        _ => "partial",
    };

    HostResult {
        host: host.to_string(),
        port: Some(port),
        test_type: "tcp".to_string(),
        attempts: count,
        successful,
        success_rate,
        avg_response_time_ms: avg_response_time,
        response_times,
        status: status.to_string(),
        error: if successful == 0 { last_error } else { None },
    }
}

async fn icmp_ping(host: String, count: u32, timeout_ms: u64) -> HostResult {
    let mut response_times = Vec::new();
    let mut successful = 0;
    let mut last_error = None;

    // Resolve hostname to IP
    let ip_addr = match host.parse::<IpAddr>() {
        Ok(ip) => ip,
        Err(_) => {
            // Try to resolve hostname
            match tokio::net::lookup_host(format!("{}:0", host)).await {
                Ok(mut addrs) => {
                    if let Some(addr) = addrs.next() {
                        addr.ip()
                    } else {
                        return HostResult {
                            host: host.to_string(),
                            port: None,
                            test_type: "icmp".to_string(),
                            attempts: count,
                            successful: 0,
                            success_rate: 0.0,
                            avg_response_time_ms: None,
                            response_times: vec![],
                            status: "down".to_string(),
                            error: Some("dns_resolution_failed".to_string()),
                        };
                    }
                }
                Err(e) => {
                    return HostResult {
                        host: host.to_string(),
                        port: None,
                        test_type: "icmp".to_string(),
                        attempts: count,
                        successful: 0,
                        success_rate: 0.0,
                        avg_response_time_ms: None,
                        response_times: vec![],
                        status: "down".to_string(),
                        error: Some(format!("dns_error: {}", e)),
                    };
                }
            }
        }
    };

    // Create ICMP client
    let config = Config::default();
    let client = match Client::new(&config) {
        Ok(client) => client,
        Err(e) => {
            return HostResult {
                host: host.to_string(),
                port: None,
                test_type: "icmp".to_string(),
                attempts: count,
                successful: 0,
                success_rate: 0.0,
                avg_response_time_ms: None,
                response_times: vec![],
                status: "down".to_string(),
                error: Some(format!("icmp_client_error: {} (try running as root/admin)", e)),
            };
        }
    };

    let mut pinger = client.pinger(ip_addr, PingIdentifier(rand::random())).await;
    pinger.timeout(Duration::from_millis(timeout_ms));

    for i in 1..=count {
        match pinger.ping(PingSequence(i as u16), &[]).await {
            Ok((IcmpPacket::V4(_packet), duration)) => {
                successful += 1;
                response_times.push(duration.as_millis());
            }
            Ok((IcmpPacket::V6(_packet), duration)) => {
                successful += 1;
                response_times.push(duration.as_millis());
            }
            Err(e) => {
                last_error = Some(format!("ping_error: {}", e));
            }
        }
    }

    let success_rate = successful as f64 / count as f64;
    let avg_response_time = if !response_times.is_empty() {
        Some(response_times.iter().sum::<u128>() as f64 / response_times.len() as f64)
    } else {
        None
    };

    let status = match success_rate {
        1.0 => "up",
        0.0 => "down",
        _ => "partial",
    };

    HostResult {
        host: host.to_string(),
        port: None,
        test_type: "icmp".to_string(),
        attempts: count,
        successful,
        success_rate,
        avg_response_time_ms: avg_response_time,
        response_times,
        status: status.to_string(),
        error: if successful == 0 { last_error } else { None },
    }
}

fn print_human_readable(results: &[HostResult]) {
    println!("\n📊 Summary");
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());

    for result in results {
        let status_icon = match result.status.as_str() {
            "up" => "✅",
            "down" => "❌",
            "partial" => "⚠️",
            _ => "❓",
        };

        let host_port = if let Some(port) = result.port {
            format!("{}:{}", result.host.blue(), port.to_string().yellow())
        } else {
            format!("{} (ICMP)", result.host.blue())
        };

        if let Some(avg_time) = result.avg_response_time_ms {
            println!(
                "{} {} → {}/{} successful (Avg: {:.2} ms) [{}]",
                     status_icon,
                     host_port,
                     result.successful,
                     result.attempts,
                     avg_time,
                     result.test_type.cyan()
            );
        } else {
            println!(
                "{} {} → {}/{} successful [{}]{}",
                status_icon,
                host_port,
                result.successful,
                result.attempts,
                result.test_type.cyan(),
                     if let Some(error) = &result.error {
                         format!(" ({})", error.red())
                     } else {
                         String::new()
                     }
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let ports = parse_ports(&args.ports);

    if args.hosts.is_empty() {
        eprintln!("{} You must provide at least one host!", "❌".red());
        return Ok(());
    }

    if !args.ping && ports.is_empty() {
        eprintln!("{} You must provide at least one port or enable --ping!", "❌".red());
        return Ok(());
    }

    if !args.json && !args.quiet {
        println!(
            "\n{} Hosts: [{}]{}{}",
            "🔍 Scanning".bold(),
                 args.hosts.join(", ").green(),
                 if !ports.is_empty() {
                     format!(", Ports: [{}]", ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ").yellow())
                 } else {
                     String::new()
                 },
                 if args.ping { format!("{}", ", ICMP Ping: enabled".magenta()) } else { String::new() }
        );
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
    }

    loop {
        let mut all_results = Vec::new();

        // Run TCP checks
        if !ports.is_empty() {
            let mut tcp_tasks = vec![];
            for host in &args.hosts {
                for &port in &ports {
                    let host_clone = host.clone();
                    let task = tcp_check(host_clone, port, args.count, args.timeout);
                    tcp_tasks.push(task);
                }
            }

            let tcp_results = future::join_all(tcp_tasks).await;
            all_results.extend(tcp_results);
        }

        // Run ICMP ping checks
        if args.ping {
            let mut ping_tasks = vec![];
            for host in &args.hosts {
                let host_clone = host.clone();
                let task = icmp_ping(host_clone, args.count, args.ping_timeout);
                ping_tasks.push(task);
            }

            let ping_results = future::join_all(ping_tasks).await;
            all_results.extend(ping_results);
        }

        // Output results
        if args.json {
            let scan_result = ScanResult {
                scan_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs()
                .to_string(),
                results: all_results,
            };
            println!("{}", serde_json::to_string_pretty(&scan_result)?);
        } else {
            print_human_readable(&all_results);
        }

        if args.once {
            break;
        }

        if !args.json && !args.quiet {
            println!("\n⏱️  Waiting 5 seconds before next scan...\n");
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    Ok(())
}
