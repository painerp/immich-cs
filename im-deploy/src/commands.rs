use anyhow::{bail, Context, Result};
use std::{
    io::{self, Write},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::config::Config;
use crate::openstack::OpenStackClient;
use crate::tailscale;
use crate::tui::{run_cloud_provider_selector, run_server_selector, CloudProvider, ServerInfo};

pub fn confirm_action(prompt: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "(Y/n)" } else { "(y/N)" };
    print!("{} {}: ", prompt, suffix);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(default_yes);
    }

    Ok(trimmed.eq_ignore_ascii_case("y"))
}


fn ensure_terraform_initialized(terraform_bin: &str, terraform_dir: &PathBuf) -> Result<()> {
    let terraform_state_dir = terraform_dir.join(".terraform");
    if !terraform_state_dir.exists() {
        println!("--- .terraform directory not found, running init first...");
        let init_status = Command::new(terraform_bin)
            .args(&["init", "-input=false"])
            .current_dir(terraform_dir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Failed to execute terraform init")?;

        if !init_status.success() {
            bail!("Terraform init failed with exit code: {:?}", init_status.code());
        }
        println!("--- Terraform init completed successfully\n");
    }
    Ok(())
}

fn run_terraform_command(terraform_bin: &str, terraform_dir: &PathBuf, args: &[&str]) -> Result<()> {
    ensure_terraform_initialized(terraform_bin, terraform_dir)?;

    let status = Command::new(terraform_bin)
        .args(args)
        .current_dir(terraform_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to execute terraform command")?;

    if !status.success() {
        bail!("Terraform command failed with exit code: {:?}", status.code());
    }

    Ok(())
}

fn get_terraform_outputs(terraform_bin: &str, terraform_dir: &PathBuf) -> Result<serde_json::Value> {
    ensure_terraform_initialized(terraform_bin, terraform_dir)?;

    let output = Command::new(terraform_bin)
        .args(&["output", "-json"])
        .current_dir(terraform_dir)
        .output()
        .context("Failed to get terraform outputs")?;

    if !output.status.success() {
        bail!("Failed to get terraform outputs");
    }

    let outputs: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("Failed to parse terraform outputs")?;

    Ok(outputs)
}

fn extract_cloud_providers(terraform_bin: &str, terraform_dir: &PathBuf) -> Result<Vec<CloudProvider>> {
    let outputs = get_terraform_outputs(terraform_bin, terraform_dir)?;

    let mut cloud_providers = Vec::new();

    // Check if Tailscale is enabled globally
    let tailscale_enabled = outputs
        .get("tailscale_enabled")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Get Tailscale hostnames if available
    let tailscale_hostnames = outputs
        .get("tailscale_hostnames")
        .and_then(|v| v.get("value"));

    // Extract OpenStack cluster
    if let Some(openstack_cluster) = outputs.get("openstack_cluster").and_then(|v| v.get("value")) {
        if !openstack_cluster.is_null() {
            let bastion_ip = openstack_cluster
                .get("bastion_ip")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut servers = Vec::new();

            // Get Tailscale hostnames for OpenStack servers and agents
            let ts_servers = if tailscale_enabled {
                tailscale_hostnames
                    .and_then(|v| v.get("openstack_servers"))
                    .and_then(|v| v.as_array())
            } else {
                None
            };

            let ts_agents = if tailscale_enabled {
                tailscale_hostnames
                    .and_then(|v| v.get("openstack_agents"))
                    .and_then(|v| v.as_array())
            } else {
                None
            };

            // Extract server IPs
            if let Some(server_ips) = openstack_cluster.get("server_ips").and_then(|v| v.as_array()) {
                for (i, ip) in server_ips.iter().enumerate() {
                    if let Some(ip_str) = ip.as_str() {
                        let tailscale_hostname = ts_servers
                            .and_then(|arr| arr.get(i))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        servers.push(ServerInfo {
                            name: format!("k3s-server-{}", i),
                            ip: ip_str.to_string(),
                            cloud_provider: "openstack".to_string(),
                            tailscale_hostname,
                        });
                    }
                }
            }

            // Extract agent IPs
            if let Some(agent_ips) = openstack_cluster.get("agent_ips").and_then(|v| v.as_array()) {
                for (i, ip) in agent_ips.iter().enumerate() {
                    if let Some(ip_str) = ip.as_str() {
                        let tailscale_hostname = ts_agents
                            .and_then(|arr| arr.get(i))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        servers.push(ServerInfo {
                            name: format!("k3s-agent-{}", i),
                            ip: ip_str.to_string(),
                            cloud_provider: "openstack".to_string(),
                            tailscale_hostname,
                        });
                    }
                }
            }

            if !servers.is_empty() {
                cloud_providers.push(CloudProvider {
                    name: "OpenStack".to_string(),
                    bastion_ip,
                    tailscale_enabled,
                    servers,
                });
            }
        }
    }

    if cloud_providers.is_empty() {
        bail!("No cloud providers found in terraform outputs. Has the cluster been deployed?");
    }

    Ok(cloud_providers)
}

pub fn cmd_deploy(config: &Config, auto_confirm: bool) -> Result<()> {
    println!("Terraform directory: {}", config.terraform_dir.display());
    println!("Using binary: {}", config.terraform_bin);
    println!();

    if !auto_confirm && !confirm_action("Are you sure you want to deploy the cluster?", false)? {
        println!("Deploy cancelled.");
        return Ok(());
    }

    println!("\nRunning terraform apply...\n");

    let apply_start = Instant::now();
    run_terraform_command(&config.terraform_bin, &config.terraform_dir, &["apply", "--auto-approve"])?;
    let apply_duration = apply_start.elapsed();

    let apply_mins = apply_duration.as_secs() / 60;
    let apply_secs = apply_duration.as_secs() % 60;

    println!("\nDeployment complete!");
    println!("Terraform apply time: {}m {:02}s\n", apply_mins, apply_secs);

    // Start monitoring timer immediately for accurate timing
    let monitor_start = Instant::now();

    // Auto-decline monitoring if -y flag was used, otherwise ask
    let should_monitor = if auto_confirm {
        println!("Skipped cluster monitoring (--yes flag)...\n");
        false
    } else {
        confirm_action("Would you like to monitor cluster formation?", true)?
    };

    if should_monitor {
        if !auto_confirm {
            println!();
        }
        cmd_monitor(config)?;
        let monitor_duration = monitor_start.elapsed();

        let monitor_mins = monitor_duration.as_secs() / 60;
        let monitor_secs = monitor_duration.as_secs() % 60;

        let total_duration = apply_duration + monitor_duration;
        let total_mins = total_duration.as_secs() / 60;
        let total_secs = total_duration.as_secs() % 60;

        println!("\nTiming Summary:");
        println!("  Terraform apply:        {}m {:02}s", apply_mins, apply_secs);
        println!("  Cluster initialization: {}m {:02}s", monitor_mins, monitor_secs);
        println!("  Total time:             {}m {:02}s", total_mins, total_secs);
    }

    Ok(())
}

pub fn cmd_destroy(config: &Config, auto_confirm: bool) -> Result<()> {
    println!("Terraform directory: {}", config.terraform_dir.display());
    println!("Using binary: {}", config.terraform_bin);
    println!();
    println!("WARNING: This will destroy all cluster resources!");
    println!();

    if !auto_confirm && !confirm_action("Are you sure you want to destroy the cluster?", false)? {
        println!("Destroy cancelled.");
        return Ok(());
    }

    // Step 1: Cleanup Tailscale devices (before terraform destroy)
    if let Some(ref ts_config) = config.tailscale {
        println!("\n=== Step 1: Cleaning up Tailscale devices ===\n");

        // Verify Tailscale connection before proceeding
        if let Err(e) = tailscale::verify_tailscale_connection() {
            eprintln!("Tailscale verification failed: {}", e);
            if !auto_confirm && !confirm_action("Continue without Tailscale cleanup?", false)? {
                println!("Destroy cancelled.");
                return Ok(());
            }
            println!("Skipping Tailscale cleanup...\n");
        } else {
            let cluster_tag = format!("{}-openstack", config.cluster_name);

            if let Err(e) = tailscale::cleanup_devices_by_tag(
                &ts_config.api_key,
                &ts_config.tailnet,
                &cluster_tag,
            ) {
                eprintln!("WARNING: Tailscale cleanup failed: {}", e);
                eprintln!("         You may need to remove devices manually from https://login.tailscale.com/admin/machines");
                eprintln!();
            }
        }
    } else {
        println!("\n=== Step 1: Tailscale cleanup skipped (not enabled) ===\n");
    }

    // Step 2: Get network ID and cluster name from terraform state before destroying
    println!("\nExtracting network_id and cluster_name from terraform state...");
    let terraform_outputs = get_terraform_outputs(&config.terraform_bin, &config.terraform_dir).ok();

    let network_id = terraform_outputs
        .as_ref()
        .and_then(|outputs| {
            outputs
                .get("openstack_cluster")
                .and_then(|v| v.get("value"))
                .and_then(|v| v.get("network_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

    let cluster_name = terraform_outputs
        .as_ref()
        .and_then(|outputs| {
            outputs
                .get("openstack_cluster")
                .and_then(|v| v.get("value"))
                .and_then(|v| v.get("cluster_name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

    if let Some(ref net_id) = network_id {
        println!("   -> Found network_id: {}", net_id);
    } else {
        println!("   WARNING: Could not extract network_id from terraform outputs");
        println!("            This may happen if:");
        println!("            1. Terraform outputs haven't been refreshed");
        println!("            2. network_id is not exposed in root outputs.tf");
        println!("            Attempting to proceed without network filtering...");
    }

    if let Some(ref cl_name) = cluster_name {
        println!("   -> Found cluster_name: {}", cl_name);
    } else {
        println!("   WARNING: Could not extract cluster_name from terraform outputs");
    }

    // Step 3: Cleanup dynamic OpenStack resources BEFORE terraform destroy
    // This is critical - dynamic LBs block terraform destroy if not removed first!
    if let Some(ref os_config) = config.openstack {
        if let Some(ref net_id) = network_id {
            if let Some(ref cl_name) = cluster_name {
                println!("\n=== Step 2: Cleaning up dynamic OpenStack resources ===");
                println!("CRITICAL: Removing dynamically created load balancers to prevent terraform destroy from blocking\n");

                match OpenStackClient::new(
                    &os_config.auth_url,
                    &os_config.username,
                    &os_config.password,
                    &os_config.project_name,
                    os_config.cacert_file.as_deref(),
                    os_config.insecure,
                ) {
                    Ok(client) => {
                        if let Err(e) = client.cleanup_before_destroy(net_id, cl_name) {
                            eprintln!("\nWARNING: Pre-destroy OpenStack cleanup failed: {}", e);
                            eprintln!("         Terraform destroy may block waiting for load balancers to be deleted.");
                            eprintln!("         You may need to manually delete LBs from OpenStack dashboard and retry.");
                            eprintln!();

                        if !confirm_action("Terraform destroy may block. Continue anyway?", false)? {
                            println!("Destroy cancelled. Please clean up load balancers manually and retry.");
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\nWARNING: Could not authenticate with OpenStack: {}", e);
                    eprintln!("         Pre-destroy cleanup skipped. Terraform destroy may block!");
                    eprintln!();

                    if !confirm_action("Terraform destroy may block without cleanup. Continue anyway?", false)? {
                        println!("Destroy cancelled.");
                        return Ok(());
                    }
                }
            }
            } else {
                println!("\n=== Step 2: OpenStack pre-cleanup skipped (cluster_name not found) ===\n");
            }
        } else {
            println!("\n=== Step 2: OpenStack pre-cleanup skipped (network_id not found) ===\n");
        }
    } else {
        println!("\n=== Step 2: OpenStack pre-cleanup skipped (credentials not available) ===\n");
    }

    // Step 4: Remove Longhorn backup container from state to preserve backups
    println!("\n=== Step 3: Preserving Longhorn backup container ===");
    println!("Removing Swift backup container from Terraform state to prevent deletion...\n");

    // Try to remove the backup container from state - ignore errors if it doesn't exist
    let state_rm_result = run_terraform_command(
        &config.terraform_bin,
        &config.terraform_dir,
        &["state", "rm", "module.openstack_k3s[0].openstack_objectstorage_container_v1.longhorn_backup[0]"],
    );

    match state_rm_result {
        Ok(_) => println!("✓ Backup container removed from state - backups will be preserved\n"),
        Err(e) => {
            // Not a critical error - container may not exist or backups may be disabled
            println!("Note: Could not remove backup container from state: {}", e);
            println!("      This is normal if Longhorn backups are disabled or container doesn't exist.\n");
        }
    }

    // Step 5: Run terraform destroy
    println!("=== Step 4: Running terraform destroy ===\n");

    let destroy_start = Instant::now();
    run_terraform_command(&config.terraform_bin, &config.terraform_dir, &["destroy", "--auto-approve"])?;
    let destroy_duration = destroy_start.elapsed();

    let destroy_mins = destroy_duration.as_secs() / 60;
    let destroy_secs = destroy_duration.as_secs() % 60;

    println!("\nTerraform destroy complete!");
    println!("Terraform destroy time: {}m {:02}s", destroy_mins, destroy_secs);

    // Step 6: Cleanup remaining orphaned OpenStack resources (after terraform destroy)
    if let Some(ref os_config) = config.openstack {
        if let Some(ref cl_name) = cluster_name {
            println!("\n=== Step 5: Cleaning up remaining orphaned OpenStack resources ===");

            match OpenStackClient::new(
                &os_config.auth_url,
                &os_config.username,
                &os_config.password,
                &os_config.project_name,
                os_config.cacert_file.as_deref(),
                os_config.insecure,
            ) {
                Ok(client) => {
                    if let Err(e) = client.cleanup_after_destroy(cl_name) {
                        eprintln!("\nWARNING: Post-destroy OpenStack cleanup failed: {}", e);
                        eprintln!("         Some resources may need to be cleaned up manually via OpenStack dashboard");
                    }
                }
                Err(e) => {
                    eprintln!("\nWARNING: Could not authenticate with OpenStack: {}", e);
                    eprintln!("         Post-destroy cleanup skipped. Check OpenStack dashboard for leftover resources.");
                }
            }
        } else {
            println!("\n=== Step 5: OpenStack post-cleanup skipped (cluster_name not found) ===");
        }
    } else {
        println!("\n=== Step 5: OpenStack post-cleanup skipped (credentials not available) ===");
    }

    println!("\nCluster destroyed!");
    Ok(())
}

pub fn cmd_ssh(config: &Config) -> Result<()> {
    println!("Fetching server information...\n");

    let cloud_providers = extract_cloud_providers(&config.terraform_bin, &config.terraform_dir)?;

    // If only one cloud provider, auto-select it
    let selected_provider = if cloud_providers.len() == 1 {
        println!("Auto-selecting {} (only provider available)\n", cloud_providers[0].name);
        cloud_providers.into_iter().next().unwrap()
    } else {
        // Show cloud provider selection
        match run_cloud_provider_selector(cloud_providers)? {
            Some(provider) => provider,
            None => {
                println!("No cloud provider selected.");
                return Ok(());
            }
        }
    };

    let servers = selected_provider.servers;

    let selected = run_server_selector(servers)?;

    if let Some(server) = selected {
        // Determine connection method: Tailscale (preferred) or bastion
        if selected_provider.tailscale_enabled {
            // Verify Tailscale connection before attempting SSH
            if let Err(e) = tailscale::verify_tailscale_connection() {
                eprintln!("\nCannot use Tailscale connection: {}", e);
                bail!("Tailscale verification failed");
            }

            if let Some(ref hostname) = server.tailscale_hostname {
                println!("\nConnecting to {} via Tailscale (hostname: {})...\n",
                    server.name, hostname);

                let status = Command::new("ssh")
                    .args(&[
                        "-o",
                        "StrictHostKeyChecking=no",
                        &format!("ubuntu@{}", hostname),
                    ])
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status()
                    .context("Failed to execute SSH")?;

                if !status.success() {
                    eprintln!("\nSSH connection via Tailscale failed!");
                    eprintln!("Troubleshooting tips:");
                    eprintln!("  1. Check if Tailscale is running on your machine: tailscale status");
                    eprintln!("  2. Verify you can resolve the hostname: ping {}", hostname);
                    eprintln!("  3. Check if the node is connected to Tailscale network");
                    bail!("SSH connection failed");
                }
            } else {
                bail!("Tailscale is enabled but hostname not found for server {}", server.name);
            }
        } else if let Some(ref bastion_ip) = selected_provider.bastion_ip {
            println!("\nConnecting to {} ({}) via bastion host {}...\n",
                server.name, server.ip, bastion_ip);

            let status = Command::new("ssh")
                .args(&[
                    "-J",
                    &format!("ubuntu@{}", bastion_ip),
                    "-o",
                    "StrictHostKeyChecking=no",
                    &format!("ubuntu@{}", server.ip),
                ])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .context("Failed to execute SSH")?;

            if !status.success() {
                bail!("SSH connection failed");
            }
        } else {
            bail!("Neither Tailscale nor bastion host available for SSH connection. Cannot connect to servers.");
        }
    } else {
        println!("No server selected.");
    }

    Ok(())
}

pub fn cmd_copy_kubeconfig(config: &Config) -> Result<()> {
    println!("Fetching cluster information...\n");

    let outputs = get_terraform_outputs(&config.terraform_bin, &config.terraform_dir)?;
    let cloud_providers = extract_cloud_providers(&config.terraform_bin, &config.terraform_dir)?;

    // Use the first available cloud provider
    let provider = cloud_providers.first()
        .context("No cloud providers found")?;

    // Get the load balancer IP from primary_api_endpoint or from specific cloud provider
    let lb_floating_ip = if let Some(endpoint) = outputs.get("primary_api_endpoint")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str()) {
        // Extract IP from https://IP:6443 format
        endpoint.trim_start_matches("https://").trim_end_matches(":6443").to_string()
    } else if provider.name == "OpenStack" {
        outputs.get("openstack_cluster")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.get("loadbalancer_ip"))
            .and_then(|v| v.as_str())
            .context("Could not get load balancer IP")?
            .to_string()
    } else {
        bail!("Could not determine load balancer IP");
    };

    // Get the first server from the provider's servers
    let server_0 = provider.servers
        .iter()
        .find(|s| s.name.contains("server"))
        .context("Could not find k3s-server-0")?;

    println!("Downloading kubeconfig from {}...", server_0.name);

    let output = if provider.tailscale_enabled {
        if let Some(ref hostname) = server_0.tailscale_hostname {
            println!("Using Tailscale connection to {}", hostname);
            Command::new("ssh")
                .args(&[
                    "-o",
                    "StrictHostKeyChecking=no",
                    &format!("ubuntu@{}", hostname),
                    "sudo cat /home/ubuntu/.kube/config",
                ])
                .output()
                .context("Failed to fetch kubeconfig via Tailscale SSH")?
        } else {
            bail!("Tailscale is enabled but hostname not found for server");
        }
    } else if let Some(ref bastion_ip) = provider.bastion_ip {
        println!("Using bastion host connection");
        Command::new("ssh")
            .args(&[
                "-J",
                &format!("ubuntu@{}", bastion_ip),
                "-o",
                "StrictHostKeyChecking=no",
                &format!("ubuntu@{}", server_0.ip),
                "sudo cat /home/ubuntu/.kube/config",
            ])
            .output()
            .context("Failed to fetch kubeconfig via bastion SSH")?
    } else {
        bail!("Neither Tailscale nor bastion host available for SSH connection");
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("SSH error: {}", stderr);
        bail!("Failed to fetch kubeconfig from server");
    }

    let kubeconfig = String::from_utf8(output.stdout)
        .context("Kubeconfig is not valid UTF-8")?;

    // Replace the server URL with the load balancer floating IP
    let kubeconfig = if let Some(start) = kubeconfig.find("server: https://") {
        let prefix = &kubeconfig[..start + 16]; // "server: https://"
        let rest = &kubeconfig[start + 16..];

        // Find the end of the IP/hostname (before :6443)
        if let Some(port_pos) = rest.find(":6443") {
            let suffix = &rest[port_pos..]; // ":6443" and everything after
            format!("{}{}{}", prefix, lb_floating_ip, suffix)
        } else {
            kubeconfig
        }
    } else {
        kubeconfig
    };

    // Write to ./kubeconfig
    let output_path = std::env::current_dir()?.join("kubeconfig");
    std::fs::write(&output_path, kubeconfig)
        .context("Failed to write kubeconfig file")?;

    println!("Kubeconfig saved to: {}", output_path.display());
    println!("\nTo use it, run:");
    println!("  export KUBECONFIG={}", output_path.display());

    Ok(())
}

pub fn cmd_monitor(config: &Config) -> Result<()> {
    println!("Fetching cluster information...\n");

    let outputs = get_terraform_outputs(&config.terraform_bin, &config.terraform_dir)?;
    let cloud_providers = extract_cloud_providers(&config.terraform_bin, &config.terraform_dir)?;

    // Use the first available cloud provider for monitoring
    let provider = cloud_providers.first()
        .context("No cloud providers found")?;

    // Verify Tailscale connection if enabled
    if provider.tailscale_enabled {
        if let Err(e) = tailscale::verify_tailscale_connection() {
            eprintln!("Tailscale verification failed: {}", e);
            bail!("Cannot monitor via Tailscale");
        }
    }

    // Get the first server
    let server_0 = provider.servers
        .iter()
        .find(|s| s.name.contains("server"))
        .context("Could not find k3s-server-0")?;

    // Count expected nodes from aggregated outputs or from cloud provider
    let server_count = outputs
        .get("all_server_ips")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or_else(|| provider.servers.iter().filter(|s| s.name.contains("server")).count());

    let agent_count = outputs
        .get("all_agent_ips")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or_else(|| provider.servers.iter().filter(|s| s.name.contains("agent")).count());

    let expected_nodes = server_count + agent_count;

    if expected_nodes == 0 {
        bail!("No nodes found in Terraform outputs. Check all_server_ips and all_agent_ips.");
    }

    // Check if GPU Operator and ArgoCD are enabled
    let gpu_enabled = outputs
        .get("enable_nvidia_gpu_operator")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let argocd_enabled = outputs
        .get("enable_argocd")
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let connection_method = if provider.tailscale_enabled {
        "Tailscale"
    } else {
        "Bastion"
    };

    println!("Monitoring k3s cluster formation...");
    println!("Connection: {} via {}", server_0.name, connection_method);
    println!("Expected nodes: {} ({} servers + {} agents)", expected_nodes, server_count, agent_count);
    if gpu_enabled {
        println!("GPU Operator: enabled");
    }
    if argocd_enabled {
        println!("ArgoCD: enabled (with Tailscale Serve)");
    }
    println!("Checking every 10 seconds");
    println!("Press Ctrl+C to stop\n");

    let start_time = Instant::now();
    let mut check_count = 0;
    let mut nodes_ready_time: Option<Duration> = None;
    let mut gpu_install_start: Option<Instant> = None;
    let mut gpu_install_complete: Option<Duration> = None;
    let mut argocd_install_start: Option<Instant> = None;
    let mut argocd_install_complete: Option<Duration> = None;
    let mut argocd_tailscale_start: Option<Instant> = None;
    let mut argocd_tailscale_complete: Option<Duration> = None;

    // Phase 1: Wait for all nodes to be Ready
    loop {
        check_count += 1;
        let elapsed = start_time.elapsed();
        let mins = elapsed.as_secs() / 60;
        let secs = elapsed.as_secs() % 60;

        // Clear screen and show status
        print!("\x1B[2J\x1B[1;1H");
        println!("=== K3s Cluster Monitor ===");
        println!("Runtime: {}m {:02}s | Check #{}", mins, secs, check_count);
        println!("Expected: {} nodes ({} servers + {} agents)", expected_nodes, server_count, agent_count);
        println!("Connection: {}", connection_method);
        println!("================================\n");

        // Try to get cluster status with appropriate connection method
        let output = if provider.tailscale_enabled {
            if let Some(ref hostname) = server_0.tailscale_hostname {
                Command::new("ssh")
                    .args(&[
                        "-o", "StrictHostKeyChecking=no",
                        "-o", "ConnectTimeout=10",
                        &format!("ubuntu@{}", hostname),
                        "sudo kubectl get nodes --no-headers 2>/dev/null",
                    ])
                    .output()
            } else {
                bail!("Tailscale hostname not found for server");
            }
        } else if let Some(ref bastion_ip) = provider.bastion_ip {
            Command::new("ssh")
                .args(&[
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "ConnectTimeout=10",
                    "-J", &format!("ubuntu@{}", bastion_ip),
                    &format!("ubuntu@{}", server_0.ip),
                    "sudo kubectl get nodes --no-headers 2>/dev/null",
                ])
                .output()
        } else {
            bail!("Neither Tailscale nor bastion host available for monitoring");
        };

        match output {
            Ok(result) if result.status.success() => {
                let nodes_output = String::from_utf8_lossy(&result.stdout);

                if nodes_output.trim().is_empty() {
                    println!("Waiting for k3s API server to be ready...");
                } else {
                    println!("Cluster Nodes:");
                    println!("{}", nodes_output);

                    // Count Ready nodes
                    let ready_count = nodes_output.lines().filter(|line| line.contains(" Ready ")).count();
                    let total_count = nodes_output.lines().count();

                    println!("Ready nodes: {}/{}", ready_count, expected_nodes);

                    if ready_count >= expected_nodes && total_count >= expected_nodes {
                        nodes_ready_time = Some(elapsed);
                        println!("\nAll {} nodes are Ready!", expected_nodes);

                        // Get detailed node info
                        let detail_output = if provider.tailscale_enabled {
                            if let Some(ref hostname) = server_0.tailscale_hostname {
                                Command::new("ssh")
                                    .args(&[
                                        "-o", "StrictHostKeyChecking=no",
                                        &format!("ubuntu@{}", hostname),
                                        "sudo kubectl get nodes -o wide",
                                    ])
                                    .output()?
                            } else {
                                bail!("Tailscale hostname not found");
                            }
                        } else if let Some(ref bastion_ip) = provider.bastion_ip {
                            Command::new("ssh")
                                .args(&[
                                    "-o", "StrictHostKeyChecking=no",
                                    "-J", &format!("ubuntu@{}", bastion_ip),
                                    &format!("ubuntu@{}", server_0.ip),
                                    "sudo kubectl get nodes -o wide",
                                ])
                                .output()?
                        } else {
                            bail!("Neither Tailscale nor bastion host available");
                        };

                        if detail_output.status.success() {
                            println!("\n{}", String::from_utf8_lossy(&detail_output.stdout));
                        }

                        let ready_mins = elapsed.as_secs() / 60;
                        let ready_secs = elapsed.as_secs() % 60;
                        println!("Cluster ready time: {}m {:02}s", ready_mins, ready_secs);
                        break;
                    }
                }
            }
            _ => {
                println!("Waiting for k3s API server to be ready...");
            }
        }

        println!("\nNext check in 10 seconds...");
        thread::sleep(Duration::from_secs(10));
    }

    // Phase 2: Monitor GPU Operator installation (if enabled)
    if gpu_enabled {
        println!("\n=== Monitoring GPU Operator Installation ===\n");
        gpu_install_start = Some(Instant::now());

        loop {
            thread::sleep(Duration::from_secs(10));

            let elapsed = start_time.elapsed();
            let mins = elapsed.as_secs() / 60;
            let secs = elapsed.as_secs() % 60;

            // Check k3s-server.log first to see if we've reached GPU installation
            let server_log_cmd = if provider.tailscale_enabled {
                if let Some(ref hostname) = server_0.tailscale_hostname {
                    Command::new("ssh")
                        .args(&[
                            "-o", "StrictHostKeyChecking=no",
                            "-o", "ConnectTimeout=10",
                            &format!("ubuntu@{}", hostname),
                            "sudo cat /var/log/k3s-server.log 2>/dev/null",
                        ])
                        .output()
                } else {
                    bail!("Tailscale hostname not found");
                }
            } else if let Some(ref bastion_ip) = provider.bastion_ip {
                Command::new("ssh")
                    .args(&[
                        "-o", "StrictHostKeyChecking=no",
                        "-o", "ConnectTimeout=10",
                        "-J", &format!("ubuntu@{}", bastion_ip),
                        &format!("ubuntu@{}", server_0.ip),
                        "sudo cat /var/log/k3s-server.log 2>/dev/null",
                    ])
                    .output()
            } else {
                bail!("Neither Tailscale nor bastion host available");
            };

            if let Ok(result) = server_log_cmd {
                if result.status.success() {
                    let server_log = String::from_utf8_lossy(&result.stdout);

                    // Check for errors in k3s-server.log
                    if server_log.contains("ERROR") || server_log.contains("FATAL") {
                        let error_lines: Vec<&str> = server_log.lines()
                            .filter(|line| line.contains("ERROR") || line.contains("FATAL"))
                            .collect();

                        if !error_lines.is_empty() {
                            println!("\nERROR detected in k3s-server.log before GPU installation!");
                            println!("Full k3s-server.log:\n");
                            println!("{}", server_log);
                            bail!("Server initialization failed");
                        }
                    }

                    // Check if GPU installation has started
                    if server_log.contains("Installing NVIDIA GPU Operator...") {
                        println!("GPU Operator installation started...");

                        // Now check the GPU operator log
                        let gpu_log_cmd = if provider.tailscale_enabled {
                            if let Some(ref hostname) = server_0.tailscale_hostname {
                                Command::new("ssh")
                                    .args(&[
                                        "-o", "StrictHostKeyChecking=no",
                                        "-o", "ConnectTimeout=10",
                                        &format!("ubuntu@{}", hostname),
                                        "sudo tail -n 5 /var/log/gpu-operator-install.log 2>/dev/null",
                                    ])
                                    .output()
                            } else {
                                bail!("Tailscale hostname not found");
                            }
                        } else if let Some(ref bastion_ip) = provider.bastion_ip {
                            Command::new("ssh")
                                .args(&[
                                    "-o", "StrictHostKeyChecking=no",
                                    "-o", "ConnectTimeout=10",
                                    "-J", &format!("ubuntu@{}", bastion_ip),
                                    &format!("ubuntu@{}", server_0.ip),
                                    "sudo tail -n 5 /var/log/gpu-operator-install.log 2>/dev/null",
                                ])
                                .output()
                        } else {
                            bail!("Neither Tailscale nor bastion host available");
                        };

                        if let Ok(log_result) = gpu_log_cmd {
                            if log_result.status.success() {
                                let gpu_log = String::from_utf8_lossy(&log_result.stdout);

                                print!("\x1B[2J\x1B[1;1H");
                                println!("=== GPU Operator Installation ===");
                                println!("Runtime: {}m {:02}s", mins, secs);
                                println!("================================\n");
                                println!("Recent log entries:");
                                println!("{}", gpu_log);

                                // Check for completion
                                if gpu_log.contains("GPU Operator installation complete!") {
                                    gpu_install_complete = Some(gpu_install_start.unwrap().elapsed());
                                    println!("\nGPU Operator installation complete!");
                                    break;
                                }

                                // Check for errors
                                if gpu_log.contains("ERROR") {
                                    println!("\nERROR detected in GPU Operator installation!");
                                    // Get full log
                                    let full_log_cmd = if provider.tailscale_enabled {
                                        if let Some(ref hostname) = server_0.tailscale_hostname {
                                            Command::new("ssh")
                                                .args(&[
                                                    "-o", "StrictHostKeyChecking=no",
                                                    &format!("ubuntu@{}", hostname),
                                                    "sudo cat /var/log/gpu-operator-install.log",
                                                ])
                                                .output()
                                        } else {
                                            bail!("Tailscale hostname not found");
                                        }
                                    } else if let Some(ref bastion_ip) = provider.bastion_ip {
                                        Command::new("ssh")
                                            .args(&[
                                                "-o", "StrictHostKeyChecking=no",
                                                "-J", &format!("ubuntu@{}", bastion_ip),
                                                &format!("ubuntu@{}", server_0.ip),
                                                "sudo cat /var/log/gpu-operator-install.log",
                                            ])
                                            .output()
                                    } else {
                                        bail!("Neither Tailscale nor bastion host available");
                                    };

                                    if let Ok(full_result) = full_log_cmd {
                                        if full_result.status.success() {
                                            println!("\nFull GPU Operator log:");
                                            println!("{}", String::from_utf8_lossy(&full_result.stdout));
                                        }
                                    }
                                    bail!("GPU Operator installation failed");
                                }

                                // Check for warnings
                                if gpu_log.contains("WARNING") {
                                    println!("\nWARNING in GPU Operator installation (continuing...)");
                                }
                            }
                        }
                    } else {
                        print!("\x1B[2J\x1B[1;1H");
                        println!("=== Waiting for GPU Operator Installation ===");
                        println!("Runtime: {}m {:02}s", mins, secs);
                        println!("===============================================\n");
                        println!("Waiting for cloud-init to reach GPU installation phase...");
                        println!("(checking k3s-server.log for 'Installing NVIDIA GPU Operator...')");
                    }
                }
            }
        }
    }

    // Phase 3: Monitor ArgoCD installation (if enabled)
    if argocd_enabled {
        println!("\n=== Monitoring ArgoCD Installation ===\n");
        argocd_install_start = Some(Instant::now());

        loop {
            thread::sleep(Duration::from_secs(10));

            let elapsed = start_time.elapsed();
            let mins = elapsed.as_secs() / 60;
            let secs = elapsed.as_secs() % 60;

            // Check k3s-server.log first to see if we've reached ArgoCD installation
            let server_log_cmd = if provider.tailscale_enabled {
                if let Some(ref hostname) = server_0.tailscale_hostname {
                    Command::new("ssh")
                        .args(&[
                            "-o", "StrictHostKeyChecking=no",
                            "-o", "ConnectTimeout=10",
                            &format!("ubuntu@{}", hostname),
                            "sudo cat /var/log/k3s-server.log 2>/dev/null",
                        ])
                        .output()
                } else {
                    bail!("Tailscale hostname not found");
                }
            } else if let Some(ref bastion_ip) = provider.bastion_ip {
                Command::new("ssh")
                    .args(&[
                        "-o", "StrictHostKeyChecking=no",
                        "-o", "ConnectTimeout=10",
                        "-J", &format!("ubuntu@{}", bastion_ip),
                        &format!("ubuntu@{}", server_0.ip),
                        "sudo cat /var/log/k3s-server.log 2>/dev/null",
                    ])
                    .output()
            } else {
                bail!("Neither Tailscale nor bastion host available");
            };

            if let Ok(result) = server_log_cmd {
                if result.status.success() {
                    let server_log = String::from_utf8_lossy(&result.stdout);

                    // Check for errors in k3s-server.log
                    if server_log.contains("ERROR") || server_log.contains("FATAL") {
                        let error_lines: Vec<&str> = server_log.lines()
                            .filter(|line| line.contains("ERROR") || line.contains("FATAL"))
                            .collect();

                        if !error_lines.is_empty() {
                            println!("\nERROR detected in k3s-server.log before ArgoCD installation!");
                            println!("Full k3s-server.log:\n");
                            println!("{}", server_log);
                            bail!("Server initialization failed");
                        }
                    }

                    // Check if ArgoCD installation has started
                    if server_log.contains("Installing ArgoCD...") {
                        println!("ArgoCD installation started...");

                        // Now check the ArgoCD log
                        let argocd_log_cmd = if provider.tailscale_enabled {
                            if let Some(ref hostname) = server_0.tailscale_hostname {
                                Command::new("ssh")
                                    .args(&[
                                        "-o", "StrictHostKeyChecking=no",
                                        "-o", "ConnectTimeout=10",
                                        &format!("ubuntu@{}", hostname),
                                        "sudo tail -n 5 /var/log/argocd-install.log 2>/dev/null",
                                    ])
                                    .output()
                            } else {
                                bail!("Tailscale hostname not found");
                            }
                        } else if let Some(ref bastion_ip) = provider.bastion_ip {
                            Command::new("ssh")
                                .args(&[
                                    "-o", "StrictHostKeyChecking=no",
                                    "-o", "ConnectTimeout=10",
                                    "-J", &format!("ubuntu@{}", bastion_ip),
                                    &format!("ubuntu@{}", server_0.ip),
                                    "sudo tail -n 5 /var/log/argocd-install.log 2>/dev/null",
                                ])
                                .output()
                        } else {
                            bail!("Neither Tailscale nor bastion host available");
                        };

                        if let Ok(log_result) = argocd_log_cmd {
                            if log_result.status.success() {
                                let argocd_log = String::from_utf8_lossy(&log_result.stdout);

                                print!("\x1B[2J\x1B[1;1H");
                                println!("=== ArgoCD Installation ===");
                                println!("Runtime: {}m {:02}s", mins, secs);
                                println!("===========================\n");
                                println!("Recent log entries:");
                                println!("{}", argocd_log);

                                // Check for completion
                                if argocd_log.contains("ArgoCD installation complete!") {
                                    argocd_install_complete = Some(argocd_install_start.unwrap().elapsed());
                                    println!("\nArgoCD installation complete!");
                                    break;
                                }

                                // Check for errors
                                if argocd_log.contains("ERROR") {
                                    println!("\nERROR detected in ArgoCD installation!");
                                    // Get full log
                                    let full_log_cmd = if provider.tailscale_enabled {
                                        if let Some(ref hostname) = server_0.tailscale_hostname {
                                            Command::new("ssh")
                                                .args(&[
                                                    "-o", "StrictHostKeyChecking=no",
                                                    &format!("ubuntu@{}", hostname),
                                                    "sudo cat /var/log/argocd-install.log",
                                                ])
                                                .output()
                                        } else {
                                            bail!("Tailscale hostname not found");
                                        }
                                    } else if let Some(ref bastion_ip) = provider.bastion_ip {
                                        Command::new("ssh")
                                            .args(&[
                                                "-o", "StrictHostKeyChecking=no",
                                                "-J", &format!("ubuntu@{}", bastion_ip),
                                                &format!("ubuntu@{}", server_0.ip),
                                                "sudo cat /var/log/argocd-install.log",
                                            ])
                                            .output()
                                    } else {
                                        bail!("Neither Tailscale nor bastion host available");
                                    };

                                    if let Ok(full_result) = full_log_cmd {
                                        if full_result.status.success() {
                                            println!("\nFull ArgoCD log:");
                                            println!("{}", String::from_utf8_lossy(&full_result.stdout));
                                        }
                                    }
                                    bail!("ArgoCD installation failed");
                                }

                                // Check for warnings
                                if argocd_log.contains("WARNING") {
                                    println!("\nWARNING in ArgoCD installation (continuing...)");
                                }
                            }
                        }
                    } else {
                        print!("\x1B[2J\x1B[1;1H");
                        println!("=== Waiting for ArgoCD Installation ===");
                        println!("Runtime: {}m {:02}s", mins, secs);
                        println!("========================================\n");
                        println!("Waiting for cloud-init to reach ArgoCD installation phase...");
                        println!("(checking k3s-server.log for 'Installing ArgoCD...')");
                    }
                }
            }
        }
    }

    // Phase 4: Monitor Tailscale ArgoCD Serve setup (if enabled)
    if argocd_enabled {
        println!("\n=== Monitoring Tailscale ArgoCD Serve Setup ===\n");
        argocd_tailscale_start = Some(Instant::now());

        loop {
            thread::sleep(Duration::from_secs(10));

            let elapsed = start_time.elapsed();
            let mins = elapsed.as_secs() / 60;
            let secs = elapsed.as_secs() % 60;

            // Check k3s-server.log first to see if we've reached Tailscale serve setup
            let server_log_cmd = if provider.tailscale_enabled {
                if let Some(ref hostname) = server_0.tailscale_hostname {
                    Command::new("ssh")
                        .args(&[
                            "-o", "StrictHostKeyChecking=no",
                            "-o", "ConnectTimeout=10",
                            &format!("ubuntu@{}", hostname),
                            "sudo cat /var/log/k3s-server.log 2>/dev/null",
                        ])
                        .output()
                } else {
                    bail!("Tailscale hostname not found");
                }
            } else if let Some(ref bastion_ip) = provider.bastion_ip {
                Command::new("ssh")
                    .args(&[
                        "-o", "StrictHostKeyChecking=no",
                        "-o", "ConnectTimeout=10",
                        "-J", &format!("ubuntu@{}", bastion_ip),
                        &format!("ubuntu@{}", server_0.ip),
                        "sudo cat /var/log/k3s-server.log 2>/dev/null",
                    ])
                    .output()
            } else {
                bail!("Neither Tailscale nor bastion host available");
            };

            if let Ok(result) = server_log_cmd {
                if result.status.success() {
                    let server_log = String::from_utf8_lossy(&result.stdout);

                    // Check for errors in k3s-server.log
                    if server_log.contains("ERROR") || server_log.contains("FATAL") {
                        let error_lines: Vec<&str> = server_log.lines()
                            .filter(|line| line.contains("ERROR") || line.contains("FATAL"))
                            .collect();

                        if !error_lines.is_empty() {
                            println!("\nERROR detected in k3s-server.log before Tailscale serve setup!");
                            println!("Full k3s-server.log:\n");
                            println!("{}", server_log);
                            bail!("Server initialization failed");
                        }
                    }

                    // Check if Tailscale serve setup has started
                    if server_log.contains("Setting up Tailscale Serve for ArgoCD...") {
                        println!("Tailscale ArgoCD Serve setup started...");

                        // Now check the tailscale-argocd-serve log
                        let serve_log_cmd = if provider.tailscale_enabled {
                            if let Some(ref hostname) = server_0.tailscale_hostname {
                                Command::new("ssh")
                                    .args(&[
                                        "-o", "StrictHostKeyChecking=no",
                                        "-o", "ConnectTimeout=10",
                                        &format!("ubuntu@{}", hostname),
                                        "sudo tail -n 5 /var/log/tailscale-argocd-serve.log 2>/dev/null",
                                    ])
                                    .output()
                            } else {
                                bail!("Tailscale hostname not found");
                            }
                        } else if let Some(ref bastion_ip) = provider.bastion_ip {
                            Command::new("ssh")
                                .args(&[
                                    "-o", "StrictHostKeyChecking=no",
                                    "-o", "ConnectTimeout=10",
                                    "-J", &format!("ubuntu@{}", bastion_ip),
                                    &format!("ubuntu@{}", server_0.ip),
                                    "sudo tail -n 5 /var/log/tailscale-argocd-serve.log 2>/dev/null",
                                ])
                                .output()
                        } else {
                            bail!("Neither Tailscale nor bastion host available");
                        };

                        if let Ok(log_result) = serve_log_cmd {
                            if log_result.status.success() {
                                let serve_log = String::from_utf8_lossy(&log_result.stdout);

                                print!("\x1B[2J\x1B[1;1H");
                                println!("=== Tailscale ArgoCD Serve Setup ===");
                                println!("Runtime: {}m {:02}s", mins, secs);
                                println!("=====================================\n");
                                println!("Recent log entries:");
                                println!("{}", serve_log);

                                // Check for completion
                                if serve_log.contains("Tailscale Serve configured successfully for ArgoCD") {
                                    argocd_tailscale_complete = Some(argocd_tailscale_start.unwrap().elapsed());
                                    println!("\nTailscale ArgoCD Serve setup complete!");

                                    // Get the full log to show access information
                                    let full_log_cmd = if provider.tailscale_enabled {
                                        if let Some(ref hostname) = server_0.tailscale_hostname {
                                            Command::new("ssh")
                                                .args(&[
                                                    "-o", "StrictHostKeyChecking=no",
                                                    &format!("ubuntu@{}", hostname),
                                                    "sudo cat /var/log/tailscale-argocd-serve.log",
                                                ])
                                                .output()
                                        } else {
                                            bail!("Tailscale hostname not found");
                                        }
                                    } else if let Some(ref bastion_ip) = provider.bastion_ip {
                                        Command::new("ssh")
                                            .args(&[
                                                "-o", "StrictHostKeyChecking=no",
                                                "-J", &format!("ubuntu@{}", bastion_ip),
                                                &format!("ubuntu@{}", server_0.ip),
                                                "sudo cat /var/log/tailscale-argocd-serve.log",
                                            ])
                                            .output()
                                    } else {
                                        bail!("Neither Tailscale nor bastion host available");
                                    };

                                    if let Ok(full_result) = full_log_cmd {
                                        if full_result.status.success() {
                                            let full_log = String::from_utf8_lossy(&full_result.stdout);
                                            // Extract the access information section
                                            if let Some(start) = full_log.find("====================================================================") {
                                                if let Some(info_section) = full_log[start..].lines().take(10).collect::<Vec<_>>().join("\n").into() {
                                                    println!("\n{}", info_section);
                                                }
                                            }
                                        }
                                    }
                                    break;
                                }

                                // Check for errors
                                if serve_log.contains("ERROR") {
                                    println!("\nERROR detected in Tailscale ArgoCD Serve setup!");
                                    // Get full log
                                    let full_log_cmd = if provider.tailscale_enabled {
                                        if let Some(ref hostname) = server_0.tailscale_hostname {
                                            Command::new("ssh")
                                                .args(&[
                                                    "-o", "StrictHostKeyChecking=no",
                                                    &format!("ubuntu@{}", hostname),
                                                    "sudo cat /var/log/tailscale-argocd-serve.log",
                                                ])
                                                .output()
                                        } else {
                                            bail!("Tailscale hostname not found");
                                        }
                                    } else if let Some(ref bastion_ip) = provider.bastion_ip {
                                        Command::new("ssh")
                                            .args(&[
                                                "-o", "StrictHostKeyChecking=no",
                                                "-J", &format!("ubuntu@{}", bastion_ip),
                                                &format!("ubuntu@{}", server_0.ip),
                                                "sudo cat /var/log/tailscale-argocd-serve.log",
                                            ])
                                            .output()
                                    } else {
                                        bail!("Neither Tailscale nor bastion host available");
                                    };

                                    if let Ok(full_result) = full_log_cmd {
                                        if full_result.status.success() {
                                            println!("\nFull Tailscale ArgoCD Serve log:");
                                            println!("{}", String::from_utf8_lossy(&full_result.stdout));
                                        }
                                    }
                                    bail!("Tailscale ArgoCD Serve setup failed");
                                }

                                // Check for warnings
                                if serve_log.contains("WARNING") {
                                    println!("\nWARNING in Tailscale ArgoCD Serve setup (continuing...)");
                                }
                            }
                        }
                    } else {
                        print!("\x1B[2J\x1B[1;1H");
                        println!("=== Waiting for Tailscale ArgoCD Serve Setup ===");
                        println!("Runtime: {}m {:02}s", mins, secs);
                        println!("=================================================\n");
                        println!("Waiting for cloud-init to reach Tailscale serve setup phase...");
                        println!("(checking k3s-server.log for 'Setting up Tailscale Serve for ArgoCD...')");
                    }
                }
            }
        }
    }

    // Final summary
    let total_time = start_time.elapsed();
    let total_mins = total_time.as_secs() / 60;
    let total_secs = total_time.as_secs() % 60;

    println!("\n\n=== Deployment Complete ===");

    if let Some(ready_time) = nodes_ready_time {
        let mins = ready_time.as_secs() / 60;
        let secs = ready_time.as_secs() % 60;
        println!("Cluster nodes ready:           {}m {:02}s", mins, secs);
    }

    if let Some(gpu_time) = gpu_install_complete {
        let mins = gpu_time.as_secs() / 60;
        let secs = gpu_time.as_secs() % 60;
        println!("GPU Operator installation:     {}m {:02}s", mins, secs);
    }

    if let Some(argocd_time) = argocd_install_complete {
        let mins = argocd_time.as_secs() / 60;
        let secs = argocd_time.as_secs() % 60;
        println!("ArgoCD installation:           {}m {:02}s", mins, secs);
    }

    if let Some(serve_time) = argocd_tailscale_complete {
        let mins = serve_time.as_secs() / 60;
        let secs = serve_time.as_secs() % 60;
        println!("ArgoCD Tailscale Serve setup:  {}m {:02}s", mins, secs);
    }

    println!("Total deployment time:         {}m {:02}s", total_mins, total_secs);
    println!("===========================\n");

    Ok(())
}

