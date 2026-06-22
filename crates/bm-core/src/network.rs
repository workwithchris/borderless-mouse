use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub ip_address: Option<String>,
    pub subnet_mask: Option<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            ip_address: Some("192.168.2.1".into()),
            subnet_mask: Some("255.255.255.0".into()),
        }
    }
}

/// Convert a dotted-quad subnet mask to CIDR prefix length.
pub fn mask_to_cidr(mask: &str) -> Option<u8> {
    let octets: Vec<u8> = mask
        .split('.')
        .filter_map(|o| o.parse().ok())
        .collect();
    if octets.len() != 4 {
        return None;
    }
    let bits: u32 = octets
        .iter()
        .fold(0u32, |acc, &o| (acc << 8) | o as u32);
    let mut count = 0u8;
    for i in (0..32).rev() {
        if (bits >> i) & 1 == 1 {
            count += 1;
        } else {
            break;
        }
    }
    // Validate no 1s after a 0
    let remaining = 32 - count;
    let mask_check = if remaining < 32 {
        (!0u32) << remaining
    } else {
        0
    };
    if bits != mask_check {
        return None;
    }
    Some(count)
}

/// Generate the shell commands needed to configure a static IP on the first
/// non-loopback interface that is link-up and lacks an IP on the target subnet.
pub fn generate_commands(config: &NetworkConfig) -> Vec<String> {
    let ip = match &config.ip_address {
        Some(v) => v.clone(),
        None => return Vec::new(),
    };
    let mask = config
        .subnet_mask
        .as_deref()
        .unwrap_or("255.255.255.0");
    let cidr = mask_to_cidr(mask).unwrap_or(24);
    let cidr_str = format!("{ip}/{cidr}");

    let mut commands = Vec::new();

    #[cfg(target_os = "linux")]
    {
        // Pick the first non-loopback, link-up interface that has no IP yet
        let iface = pick_interface_linux().unwrap_or_else(|| "eth0".into());
        commands.push(format!("sudo ip addr add {cidr_str} dev {iface}"));
        commands.push(format!("sudo ip link set {iface} up"));
    }

    #[cfg(target_os = "macos")]
    {
        let iface = pick_interface_macos().unwrap_or_else(|| "en0".into());
        commands.push(format!("sudo ifconfig {iface} {ip} netmask {mask} up"));
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (ip, mask, cidr);
        commands.push("echo \"unsupported OS — configure IP manually\"".into());
    }

    commands
}

/// Detect non-loopback, link-up interfaces that might be on a direct link.
pub fn detect_interfaces() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        detect_interfaces_linux()
    }
    #[cfg(target_os = "macos")]
    {
        detect_interfaces_macos()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Vec::new()
    }
}

/// Run the generated commands via `pkexec` on Linux or `osascript` on macOS.
/// Returns Ok with the platform-specific command string, or Err with details.
pub fn apply_config_via_gui(config: &NetworkConfig) -> Result<String, String> {
    let commands = generate_commands(config);
    if commands.is_empty() {
        return Err("no commands to run — IP address is empty".into());
    }

    #[cfg(target_os = "linux")]
    {
        let script = commands.join(" && ");
        match Command::new("pkexec")
            .args(["sh", "-c", &script])
            .output()
        {
            Ok(out) if out.status.success() => {
                Ok(format!("Applied:\n{}", commands.join("\n")))
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                Err(format!("pkexec failed: {}", stderr.trim()))
            }
            Err(e) => Err(format!("pkexec not found: {e} — run manually:\n{}", commands.join("\n"))),
        }
    }

    #[cfg(target_os = "macos")]
    {
        let script = commands.join(" && ");
        match Command::new("osascript")
            .args([
                "-e",
                &format!("do shell script \"{}\" with administrator privileges", script),
            ])
            .output()
        {
            Ok(out) if out.status.success() => {
                Ok(format!("Applied:\n{}", commands.join("\n")))
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                Err(format!("osascript failed: {}", stderr.trim()))
            }
            Err(e) => Err(format!("osascript not found: {e} — run manually:\n{}", commands.join("\n"))),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Err(format!(
            "unsupported OS — run manually:\n{}",
            commands.join("\n")
        ))
    }
}

// -- Linux platform helpers --

#[cfg(target_os = "linux")]
fn pick_interface_linux() -> Option<String> {
    detect_interfaces_linux().into_iter().next()
}

#[cfg(target_os = "linux")]
fn detect_interfaces_linux() -> Vec<String> {
    let output = Command::new("ip").args(["-o", "link", "show"]).output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut candidates: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            let name = line.split(':').nth(1)?.trim().to_string();
            if name == "lo" {
                return None;
            }
            if !line.contains("UP") && !line.contains("state UP") {
                return None;
            }
            Some(name)
        })
        .collect();
    candidates.sort();
    candidates
}

// -- macOS platform helpers --

#[cfg(target_os = "macos")]
fn pick_interface_macos() -> Option<String> {
    detect_interfaces_macos().into_iter().next()
}

#[cfg(target_os = "macos")]
fn detect_interfaces_macos() -> Vec<String> {
    let output = Command::new("ifconfig").arg("-l").output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let all: Vec<&str> = stdout.trim().split_whitespace().collect();
    let mut candidates: Vec<String> = all
        .iter()
        .filter(|name| **name != "lo0")
        .filter(|name| {
            Command::new("ifconfig")
                .arg(name)
                .output()
                .ok()
                .map_or(false, |info| {
                    let out = String::from_utf8_lossy(&info.stdout);
                    out.contains("status: active")
                })
        })
        .map(|s| s.to_string())
        .collect();
    candidates.sort();
    candidates
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_to_cidr_24() {
        assert_eq!(mask_to_cidr("255.255.255.0"), Some(24));
    }

    #[test]
    fn mask_to_cidr_16() {
        assert_eq!(mask_to_cidr("255.255.0.0"), Some(16));
    }

    #[test]
    fn mask_to_cidr_8() {
        assert_eq!(mask_to_cidr("255.0.0.0"), Some(8));
    }

    #[test]
    fn mask_to_cidr_32() {
        assert_eq!(mask_to_cidr("255.255.255.255"), Some(32));
    }

    #[test]
    fn mask_to_cidr_0() {
        assert_eq!(mask_to_cidr("0.0.0.0"), Some(0));
    }

    #[test]
    fn mask_to_cidr_invalid_chars() {
        assert_eq!(mask_to_cidr("not-a-mask"), None);
    }

    #[test]
    fn mask_to_cidr_invalid_octets() {
        assert_eq!(mask_to_cidr("255.255.256.0"), None);
    }

    #[test]
    fn mask_to_cidr_discontiguous() {
        // 255.255.255.1 is not a valid subnet mask (discontiguous)
        assert_eq!(mask_to_cidr("255.255.255.1"), None);
    }

    #[test]
    fn generate_commands_empty_config() {
        let config = NetworkConfig {
            ip_address: None,
            subnet_mask: None,
        };
        let cmds = generate_commands(&config);
        assert!(cmds.is_empty());
    }

    #[test]
    fn generate_commands_with_defaults() {
        let config = NetworkConfig::default();
        let cmds = generate_commands(&config);
        // Should produce at least one command
        assert!(!cmds.is_empty());
        // Default IP is 192.168.2.1
        assert!(cmds.iter().any(|c| c.contains("192.168.2.1")));
    }

    #[test]
    fn network_config_toml_roundtrip() {
        let config = NetworkConfig {
            ip_address: Some("10.0.0.1".into()),
            subnet_mask: Some("255.0.0.0".into()),
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: NetworkConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.ip_address.unwrap(), "10.0.0.1");
        assert_eq!(parsed.subnet_mask.unwrap(), "255.0.0.0");
    }

    #[test]
    fn mask_to_cidr_27() {
        assert_eq!(mask_to_cidr("255.255.255.224"), Some(27));
    }

    #[test]
    fn mask_to_cidr_29() {
        assert_eq!(mask_to_cidr("255.255.255.248"), Some(29));
    }
}
