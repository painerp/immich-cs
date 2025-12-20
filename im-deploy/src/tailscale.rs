use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Device {
    id: String,
    name: String,
    tags: Vec<String>,
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
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct CurrentTailnet {
    #[serde(rename = "Name")]
    name: String,
}

pub fn cleanup_devices_by_tag(api_key: &str, tailnet: &str, cluster_tag: &str) -> Result<()> {
    println!("Searching for Tailscale devices with tag: {}", cluster_tag);

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // List all devices
    let url = format!("https://api.tailscale.com/api/v2/tailnet/{}/devices", tailnet);
    let response = client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .context("Failed to list Tailscale devices")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Tailscale API error ({}): {}",
            status,
            body
        ));
    }

    let devices_response: DevicesResponse = response
        .json()
        .context("Failed to parse Tailscale devices response")?;

    // Filter devices by cluster tag
    let matching_devices: Vec<&Device> = devices_response
        .devices
        .iter()
        .filter(|d| d.tags.iter().any(|t| t == &format!("tag:{}", cluster_tag)))
        .collect();

    if matching_devices.is_empty() {
        println!("  -> No Tailscale devices found with tag '{}'", cluster_tag);
        return Ok(());
    }

    println!("  Found {} device(s) to delete:", matching_devices.len());
    for device in &matching_devices {
        println!("    - {} ({})", device.name, device.id);
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
                println!("    -> Deleted device: {}", device.name);
                deleted_count += 1;
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                eprintln!("    ERROR: Failed to delete {}: {} - {}", device.name, status, body);
                failed_count += 1;
            }
            Err(e) => {
                eprintln!("    ERROR: Failed to delete {}: {}", device.name, e);
                failed_count += 1;
            }
        }
    }

    println!("\nTailscale cleanup complete: {} deleted, {} failed", deleted_count, failed_count);

    if failed_count > 0 {
        println!("WARNING: Some devices could not be deleted. You may need to remove them manually from the Tailscale admin console.");
    }

    Ok(())
}

pub fn verify_tailscale_connection() -> Result<()> {
    // Check if tailscale is installed
    let which_status = Command::new("which")
        .arg("tailscale")
        .output();

    if which_status.is_err() || !which_status.unwrap().status.success() {
        eprintln!("WARNING: Tailscale CLI not found on this system");
        eprintln!("         Please install Tailscale to use Tailscale features");
        return Err(anyhow::anyhow!("Tailscale CLI not installed"));
    }

    // Get tailscale status
    let status_output = Command::new("tailscale")
        .args(&["status", "--json"])
        .output()
        .context("Failed to execute 'tailscale status --json'")?;

    if !status_output.status.success() {
        eprintln!("WARNING: Failed to get Tailscale status");
        eprintln!("         Make sure Tailscale is running: sudo systemctl start tailscaled");
        return Err(anyhow::anyhow!("Failed to get Tailscale status"));
    }

    let status: TailscaleStatus = serde_json::from_slice(&status_output.stdout)
        .context("Failed to parse Tailscale status JSON")?;

    // Check if Tailscale is running
    if status.backend_state != "Running" {
        eprintln!("WARNING: Tailscale is not running (state: {})", status.backend_state);
        eprintln!("         Please start Tailscale: sudo tailscale up");
        return Err(anyhow::anyhow!("Tailscale is not running"));
    }

    // Check if connected to the correct tailnet
    if let Some(tailnet) = status.current_tailnet {
        if tailnet.name != "cloudserv11.github" {
            eprintln!("WARNING: Connected to wrong Tailscale account");
            eprintln!("         Current account: {}", tailnet.name);
            eprintln!("         Expected account: cloudserv11.github");
            eprintln!();

            print!("Would you like to switch to cloudserv11.github? (y/N): ");
            std::io::Write::flush(&mut std::io::stdout())?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if input.trim().eq_ignore_ascii_case("y") {
                println!("Switching Tailscale account to cloudserv11.github...");
                let switch_status = Command::new("sudo")
                    .args(&["tailscale", "switch", "cloudserv11.github"])
                    .status()
                    .context("Failed to switch Tailscale account")?;

                if !switch_status.success() {
                    return Err(anyhow::anyhow!("Failed to switch Tailscale account"));
                }

                println!("Successfully switched to cloudserv11.github");
            } else {
                println!("Continuing with current account (operations may fail)...");
            }
        }
    } else {
        eprintln!("WARNING: Could not determine current Tailscale account");
        eprintln!("         Please verify you are connected to cloudserv11.github");
    }

    Ok(())
}

