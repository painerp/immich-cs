use crate::constants::network;
use crate::errors::{Result, TailscaleError};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::process::Command;
use tracing::{debug, info, warn};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Device {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    hostname: String,
    #[serde(default)]
    tags: Vec<String>,
}

impl Device {
    fn display_name(&self) -> &str {
        if !self.name.is_empty() {
            &self.name
        } else if !self.hostname.is_empty() {
            &self.hostname
        } else {
            &self.id
        }
    }
}

#[derive(Debug, Deserialize)]
struct DevicesResponse {
    devices: Vec<Device>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct TailscaleStatus {
    #[serde(rename = "BackendState")]
    backend_state: String,
    #[serde(rename = "CurrentTailnet")]
    current_tailnet: Option<CurrentTailnet>,
    #[serde(rename = "MagicDNSSuffix")]
    magic_dns_suffix: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct CurrentTailnet {
    #[serde(rename = "Name")]
    name: String,
}

pub fn cleanup_devices_by_tag(api_key: &str, tailnet: &str, cluster_tag: &str) -> Result<()> {
    info!("Searching for Tailscale devices with tag: {}", cluster_tag);

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(network::HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| TailscaleError::ApiError(e.to_string()))?;

    // List all devices
    let url = format!("https://api.tailscale.com/api/v2/tailnet/{}/devices", tailnet);
    let response = client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .map_err(|e| TailscaleError::ApiError(format!("Failed to list devices: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(TailscaleError::ApiError(format!(
            "API returned {}: {}",
            status,
            body
        )).into());
    }

    // Get response text first for better error handling
    let response_text = response
        .text()
        .map_err(|e| TailscaleError::ApiError(format!("Failed to read response: {}", e)))?;

    // Try to parse as JSON
    let devices_response: DevicesResponse = serde_json::from_str(&response_text)
        .map_err(|e| TailscaleError::ParseError(format!("{}: {}", e, response_text)))?;

    // Filter devices by cluster tag
    let matching_devices: Vec<&Device> = devices_response
        .devices
        .iter()
        .filter(|d| d.tags.iter().any(|t| t == &format!("tag:{}", cluster_tag)))
        .collect();

    if matching_devices.is_empty() {
        info!("No Tailscale devices found with tag '{}'", cluster_tag);
        return Ok(());
    }

    info!("Found {} device(s) to delete:", matching_devices.len());
    for device in &matching_devices {
        info!("  - {} ({})", device.display_name(), device.id);
    }

    // Delete each device
    let mut deleted_count = 0;
    let mut failed_count = 0;

    for device in matching_devices {
        let delete_url = format!("https://api.tailscale.com/api/v2/device/{}", device.id);
        match client
            .delete(&delete_url)
            .bearer_auth(api_key)
            .send()
        {
            Ok(resp) if resp.status().is_success() => {
                info!("Deleted device: {}", device.display_name());
                deleted_count += 1;
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                warn!("Failed to delete {}: {} - {}", device.display_name(), status, body);
                failed_count += 1;
            }
            Err(e) => {
                warn!("Failed to delete {}: {}", device.display_name(), e);
                failed_count += 1;
            }
        }
    }

    info!("Tailscale cleanup complete: {} deleted, {} failed", deleted_count, failed_count);

    if failed_count > 0 {
        warn!("Some devices could not be deleted. You may need to remove them manually from the Tailscale admin console.");
    }

    Ok(())
}

pub fn verify_tailscale_connection(expected_tailnet: Option<&str>) -> Result<()> {
    debug!("Verifying Tailscale connection");

    // Check if tailscale is installed
    let which_status = Command::new("which")
        .arg("tailscale")
        .output();

    if which_status.is_err() || !which_status.unwrap().status.success() {
        warn!("Tailscale CLI not found on this system");
        return Err(TailscaleError::CliNotInstalled.into());
    }

    // Get tailscale status
    let status_output = Command::new("tailscale")
        .args(&["status", "--json"])
        .output()
        .map_err(|e| TailscaleError::ApiError(format!("Failed to execute 'tailscale status': {}", e)))?;

    if !status_output.status.success() {
        warn!("Failed to get Tailscale status. Make sure Tailscale is running: sudo systemctl start tailscaled");
        return Err(TailscaleError::NotRunning("unknown".to_string()).into());
    }

    let status: TailscaleStatus = serde_json::from_slice(&status_output.stdout)
        .map_err(|e| TailscaleError::ParseError(format!("Failed to parse status JSON: {}", e)))?;

    // Check if Tailscale is running
    if status.backend_state != "Running" {
        warn!("Tailscale is not running (state: {}). Please start Tailscale: sudo tailscale up", status.backend_state);
        return Err(TailscaleError::NotRunning(status.backend_state).into());
    }

    // Check if connected to the correct tailnet (if expected_tailnet is provided)
    if let (Some(expected), Some(current_tailnet)) = (expected_tailnet, status.current_tailnet) {
        if current_tailnet.name != expected {
            warn!("Connected to wrong Tailscale account. Current: {}, Expected: {}", current_tailnet.name, expected);

            print!("Would you like to switch to {}? (y/N): ", expected);
            std::io::Write::flush(&mut std::io::stdout())?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if input.trim().eq_ignore_ascii_case("y") {
                info!("Switching Tailscale account to {}...", expected);
                let switch_status = Command::new("sudo")
                    .args(&["tailscale", "switch", expected])
                    .status()
                    .map_err(|_| TailscaleError::AccountSwitchFailed)?;

                if !switch_status.success() {
                    return Err(TailscaleError::AccountSwitchFailed.into());
                }
                info!("Successfully switched to {}", expected);
            } else {
                warn!("Continuing with current account (operations may fail)...");
            }
        }
    }

    debug!("Tailscale connection verified");
    Ok(())
}

/// Get the Tailscale MagicDNS suffix for URL construction
/// Returns an error if Tailscale is not running or MagicDNS is not available
pub fn get_magic_dns_suffix() -> Result<String> {
    debug!("Retrieving Tailscale MagicDNS suffix");

    // Check if tailscale is installed
    let which_status = Command::new("which")
        .arg("tailscale")
        .output();

    if which_status.is_err() || !which_status.unwrap().status.success() {
        return Err(TailscaleError::CliNotInstalled.into());
    }

    // Get tailscale status
    let status_output = Command::new("tailscale")
        .args(&["status", "--json"])
        .output()
        .map_err(|e| TailscaleError::ApiError(format!("Failed to execute 'tailscale status': {}", e)))?;

    if !status_output.status.success() {
        return Err(TailscaleError::NotRunning("unknown".to_string()).into());
    }

    let status: TailscaleStatus = serde_json::from_slice(&status_output.stdout)
        .map_err(|e| TailscaleError::ParseError(format!("Failed to parse status JSON: {}", e)))?;

    // Check if Tailscale is running
    if status.backend_state != "Running" {
        return Err(TailscaleError::NotRunning(status.backend_state).into());
    }

    // Get MagicDNS suffix
    status.magic_dns_suffix
        .ok_or_else(|| TailscaleError::ApiError("MagicDNS suffix not available. Ensure MagicDNS is enabled in your Tailscale settings.".to_string()).into())
}

/// Get Tailscale serve URL for a service hostname
pub fn get_tailscale_url(hostname: &str) -> Result<String> {
    let dns_suffix = get_magic_dns_suffix()?;
    Ok(format!("https://{}.{}", hostname, dns_suffix))
}

/// Get all Tailscale hostnames from kubernetes services
/// This queries services with tailscale.com/hostname annotation
pub fn get_tailscale_hostnames_from_k8s(
    connection: &crate::domain::connection::ConnectionStrategy,
) -> Result<Vec<(String, String)>> {
    use crate::domain::services::execute_kubectl_command;

    // Get all services with tailscale.com/hostname annotation
    let output = execute_kubectl_command(
        connection,
        r#"get services -A -o json"#,
    )?;

    let services: serde_json::Value = serde_json::from_str(&output)
        .map_err(|e| TailscaleError::ParseError(format!("Failed to parse kubectl output: {}", e)))?;

    let mut hostnames = Vec::new();

    if let Some(items) = services.get("items").and_then(|v| v.as_array()) {
        for item in items {
            if let (Some(metadata), Some(annotations)) = (
                item.get("metadata"),
                item.get("metadata").and_then(|m| m.get("annotations"))
            ) {
                if let Some(hostname) = annotations.get("tailscale.com/hostname").and_then(|h| h.as_str()) {
                    if let Some(namespace) = metadata.get("namespace").and_then(|n| n.as_str()) {
                        hostnames.push((namespace.to_string(), hostname.to_string()));
                    }
                }
            }
        }
    }

    Ok(hostnames)
}

