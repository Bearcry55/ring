# 🔍 RING - Rust Internet Network Grapher

[

> **A blazingly fast, parallel network connectivity scanner written in Rust**

RING is a modern alternative to traditional `ping` that combines **TCP port scanning** and **ICMP ping** capabilities with **parallel execution** and **JSON output** for seamless automation and monitoring.

## ✨ Features

- 🚀 **Parallel Scanning** - Scan multiple hosts and ports simultaneously
- 🌐 **Dual Protocol Support** - Both TCP connectivity and ICMP ping
- ⚡ **Lightning Fast** - Async/await architecture for maximum performance  
- 📊 **JSON Output** - Perfect for automation and monitoring systems
- 🎯 **Port Ranges** - Support for individual ports and ranges (e.g., `80,443,8000-8100`)
- 🔄 **Flexible Modes** - Single scan or continuous monitoring
- 🎨 **Beautiful Output** - Colored, emoji-rich terminal output
- 🛠️ **Highly Configurable** - Custom timeouts, retry counts, and more

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/Bearcry55/ring.git
cd ring

# Build and install
cargo install --path .

# Or run directly
cargo run -- --help
```

### Basic Usage

```bash
# Simple connectivity check
ring google.com

# Multiple hosts with custom ports
ring google.com cloudflare.com -p 80,443,8080

# Include ICMP ping
ring api.example.com --ping -p 80,443

# JSON output for automation
ring server1.com server2.com --json --once

# Port range scanning
ring target.com -p 8000-8100 --once
```

## 📖 Usage Examples

### 🌍 Multi-Host Health Check
```bash
# Check multiple services in parallel
ring api.example.com db.example.com cache.example.com \
  -p 80,443,5432,6379 --ping --json --once
```

### 🔄 Continuous Monitoring
```bash
# Monitor critical services every 5 seconds
ring production-server.com -p 80,443,8080 --ping -c 5
```

### 📊 Infrastructure Scanning
```bash
# Scan entire port range on multiple hosts
ring 192.168.1.10 192.168.1.20 -p 1-1000 --once --quiet
```

### 🤖 CI/CD Integration
```bash
# Health check before deployment
ring api.staging.com --ping -p 80,443 --json --once | \
  jq -e '.results[] | select(.status != "up") | length == 0'
```

## 🛠️ Command Line Options

```
Usage: ring [OPTIONS] <HOSTS>...

Arguments:
  <HOSTS>...  One or more hostnames or IPs

Options:
  -p, --ports <PORTS>              Ports (comma-separated or range) [default: 80]
  -c, --count <COUNT>              Number of attempts per host+port [default: 3]
  -t, --timeout <TIMEOUT>          Connection timeout in milliseconds [default: 2000]
  -q, --quiet                      Suppress individual result lines
  -j, --json                       Output results in JSON format
  -i, --once                       Run once instead of continuously
      --ping                       Enable ICMP ping
      --ping-timeout <TIMEOUT>     ICMP ping timeout in milliseconds [default: 1000]
  -h, --help                       Print help
  -V, --version                    Print version
```

## 📊 Output Formats

### Human-Readable Output
```
🔍 Scanning Hosts: [google.com, cloudflare.com], Ports: [80, 443], ICMP Ping: enabled
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📊 Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ google.com:80 → 3/3 successful (Avg: 45.20 ms) [tcp]
✅ google.com:443 → 3/3 successful (Avg: 38.70 ms) [tcp]
✅ google.com (ICMP) → 3/3 successful (Avg: 12.30 ms) [icmp]
```

### JSON Output
```json
{
  "scan_timestamp": "1719403800",
  "results": [
    {
      "host": "google.com",
      "port": 80,
      "test_type": "tcp",
      "attempts": 3,
      "successful": 3,
      "success_rate": 1.0,
      "avg_response_time_ms": 45.2,
      "response_times": [44, 46, 46],
      "status": "up",
      "error": null
    }
  ]
}
```

## ⚡ Performance Comparison

| Tool | Hosts | Ports | Time | Parallel |
|------|-------|-------|------|----------|
| Traditional ping | 5 hosts | N/A | ~15s | ❌ |
| RING | 5 hosts | 3 ports each | ~3s | ✅ |
| **Speed Improvement** | | | **5x faster** | |

## 🔧 Advanced Usage

### Automation with jq
```bash
# Find all failed services
ring api.example.com -p 80,443,8080 --json --once | \
  jq '.results[] | select(.status == "down")'

# Get average response times
ring fast-server.com --json --once | \
  jq '.results[].avg_response_time_ms'

# Monitor critical services and alert on failure
ring critical-api.com --json | \
  jq -r 'select(.results[] | .status == "down") | "ALERT: Service down!"'
```

### Integration Examples

#### Prometheus Monitoring
```bash
# Export metrics for Prometheus
ring api.example.com --json --once | \
  jq -r '.results[] | "ring_response_time{host=\"\(.host)\",port=\"\(.port)\"} \(.avg_response_time_ms // 0)"'
```

#### Shell Scripting
```bash
#!/bin/bash
# Health check script
if ring $1 --ping --json --once | jq -e '.results[] | select(.status != "up") | length == 0' >/dev/null; then
  echo "✅ All services healthy"
  exit 0
else
  echo "❌ Some services are down"
  exit 1
fi
```

## 🏗️ Building from Source

### Prerequisites
- Rust 1.70 or higher
- Cargo package manager

### Dependencies
```toml
[dependencies]
clap = { version = "4.4", features = ["derive"] }
colored = "2.1"
tokio = { version = "1.35", features = ["full"] }
futures = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
surge-ping = "0.8"
rand = "0.8"
```

### Build Commands
```bash
# Development build
cargo build

# Optimized release build
cargo build --release

# Run tests
cargo test

# Install globally
cargo install --path .
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup
```bash
git clone https://github.com/Bearcry55/ring.git
cd ring
cargo build
cargo test
```

## 📝 License

This project is licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) for performance and safety
- Uses [Tokio](https://tokio.rs/) for async networking
- CLI powered by [Clap](https://clap.rs/)
- ICMP implementation using [surge-ping](https://crates.io/crates/surge-ping)

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/Bearcry55/ring/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Bearcry55/ring/discussions)

---

<div align="center">

**Made with ❤️ and ☕ by [Bearcry55](https://github.com/Bearcry55)**

⭐ **Star this repo if you find it useful!** ⭐

</div>
