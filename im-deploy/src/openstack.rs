use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: Token,
}

#[derive(Debug, Deserialize)]
struct Token {
    #[serde(rename = "catalog")]
    catalog: Vec<CatalogEntry>,
    project: Option<ProjectInfo>,
}

#[derive(Debug, Deserialize)]
struct CatalogEntry {
    #[serde(rename = "type")]
    service_type: String,
    endpoints: Vec<Endpoint>,
}

#[derive(Debug, Deserialize)]
struct Endpoint {
    url: String,
    interface: String,
    region: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectInfo {
    id: String,
}

#[derive(Debug, Serialize)]
struct AuthRequest {
    auth: Auth,
}

#[derive(Debug, Serialize)]
struct Auth {
    identity: Identity,
    scope: Scope,
}

#[derive(Debug, Serialize)]
struct Identity {
    methods: Vec<String>,
    password: Password,
}

#[derive(Debug, Serialize)]
struct Password {
    user: User,
}

#[derive(Debug, Serialize)]
struct User {
    name: String,
    domain: Domain,
    password: String,
}

#[derive(Debug, Serialize)]
struct Domain {
    name: String,
}

#[derive(Debug, Serialize)]
struct Scope {
    project: Project,
}

#[derive(Debug, Serialize)]
struct Project {
    name: String,
    domain: Domain,
}

#[derive(Debug, Deserialize)]
struct FloatingIP {
    id: String,
    floating_ip_address: String,
    status: String,
    port_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FloatingIPsResponse {
    floatingips: Vec<FloatingIP>,
}

#[derive(Debug, Deserialize)]
struct Port {
    id: String,
    name: String,
    device_owner: String,
    network_id: String,
}

#[derive(Debug, Deserialize)]
struct PortsResponse {
    ports: Vec<Port>,
}

#[derive(Debug, Deserialize)]
struct LoadBalancer {
    id: String,
    name: String,
    vip_network_id: String,
    provisioning_status: String,
}

#[derive(Debug, Deserialize)]
struct LoadBalancersResponse {
    loadbalancers: Vec<LoadBalancer>,
}

#[derive(Debug, Deserialize)]
pub struct Volume {
    pub id: String,
    pub name: String,
    pub size: u32,
    pub status: String,
}

#[derive(Debug, Deserialize)]
struct VolumesResponse {
    volumes: Vec<Volume>,
}

#[derive(Debug, Deserialize)]
struct SecurityGroup {
    id: String,
    name: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct SecurityGroupsResponse {
    security_groups: Vec<SecurityGroup>,
}

pub struct OpenStackClient {
    client: Client,
    auth_token: String,
    neutron_endpoint: String,
    octavia_endpoint: String
}

impl OpenStackClient {

    pub fn new(
        auth_url: &str,
        username: &str,
        password: &str,
        project_name: &str,
        cacert_file: Option<&str>,
        insecure: bool,
    ) -> Result<Self> {
        println!("Authenticating with OpenStack...");

        let mut client_builder = Client::builder()
            .timeout(std::time::Duration::from_secs(30));

        // Handle certificate validation
        if insecure {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        } else if let Some(cert_path) = cacert_file {
            let cert_data = fs::read(cert_path)
                .with_context(|| format!("Failed to read CA certificate from {}", cert_path))?;
            let cert = reqwest::Certificate::from_pem(&cert_data)?;
            client_builder = client_builder.add_root_certificate(cert);
        }

        let client = client_builder.build()?;

        // Authenticate with Keystone
        let auth_request = AuthRequest {
            auth: Auth {
                identity: Identity {
                    methods: vec!["password".to_string()],
                    password: Password {
                        user: User {
                            name: username.to_string(),
                            domain: Domain {
                                name: "Default".to_string(),
                            },
                            password: password.to_string(),
                        },
                    },
                },
                scope: Scope {
                    project: Project {
                        name: project_name.to_string(),
                        domain: Domain {
                            name: "Default".to_string(),
                        },
                    },
                },
            },
        };

        let auth_endpoint = format!("{}/auth/tokens", auth_url);
        let response = client
            .post(&auth_endpoint)
            .json(&auth_request)
            .send()
            .context("Failed to authenticate with OpenStack")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow::anyhow!(
                "OpenStack authentication failed ({}): {}",
                status,
                body
            ));
        }

        let auth_token = response
            .headers()
            .get("X-Subject-Token")
            .context("No X-Subject-Token in response")?
            .to_str()
            .context("Invalid X-Subject-Token header")?
            .to_string();

        let token_data: TokenResponse = response
            .json()
            .context("Failed to parse authentication response")?;

        let neutron_endpoint = auth_url.replace(":5000/v3", ":9696/v2.0");
        let octavia_endpoint = auth_url.replace(":5000/v3", ":9876/v2.0");

        println!("  -> Authenticated successfully\n");

        Ok(Self {
            client,
            auth_token,
            neutron_endpoint,
            octavia_endpoint,
        })
    }

    pub fn cleanup_before_destroy(&self, network_id: &str, _cluster_name: &str) -> Result<()> {
        println!("\n=== Pre-Destroy Cleanup ===");
        println!("Removing dynamic resources to prevent terraform destroy from blocking...\n");

        self.cleanup_loadbalancers(network_id)?;

        // Manually delete Octavia ports after LB deletion
        // Cascade delete should handle this, but sometimes ports linger
        self.cleanup_octavia_ports(network_id)?;

        println!("\n=== Pre-destroy cleanup complete ===");
        println!("Terraform destroy can now proceed safely.\n");
        Ok(())
    }

    pub fn cleanup_after_destroy(&self, cluster_name: &str) -> Result<()> {
        println!("\n=== Post-Destroy Cleanup ===");
        println!("Cleaning up remaining orphaned resources...\n");

        self.cleanup_floating_ips()?;
        self.cleanup_loadbalancer_ports()?;

        // Security groups must be deleted last, after all resources using them are gone
        self.cleanup_security_groups(cluster_name)?;

        Ok(())
    }

    pub fn cleanup_orphaned_resources(&self, network_id: Option<&str>) -> Result<()> {
        println!("\n=== Cleanup Orphaned Resources ===\n");

        self.cleanup_floating_ips()?;
        self.cleanup_loadbalancer_ports()?;

        if let Some(net_id) = network_id {
            self.cleanup_loadbalancers(net_id)?;
            self.cleanup_network_ports(net_id)?;
        }

        Ok(())
    }

    fn cleanup_loadbalancers(&self, network_id: &str) -> Result<()> {
        println!("Checking for dynamically created load balancers...");

        let url = format!("{}/lbaas/loadbalancers", self.octavia_endpoint);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .context("Failed to list load balancers")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            eprintln!("WARNING: Failed to list load balancers ({}): {}", status, body);
            return Ok(());
        }

        let lbs_response: LoadBalancersResponse = response
            .json()
            .context("Failed to parse load balancers response")?;

        // Filter load balancers by network_id AND exclude terraform-managed ones
        // K8s creates LBs with names like: kube_service_<namespace>_<service>_<uuid>
        // Terraform creates LBs with names like: {cluster_name}-lb
        // We only want to delete k8s-created LBs
        let network_lbs: Vec<&LoadBalancer> = lbs_response
            .loadbalancers
            .iter()
            .filter(|lb| {
                // Must be on the cluster network
                lb.vip_network_id == network_id
                // Must be a k8s-created LB (starts with "kube" or "kube_service")
                && (lb.name.starts_with("kube_service_") || lb.name.starts_with("kube-"))
                // Explicitly exclude terraform-managed LB (ends with "-lb")
                && !lb.name.ends_with("-lb")
            })
            .collect();

        if network_lbs.is_empty() {
            println!("  -> No dynamically created load balancers found on network {}", network_id);
            println!("     (Terraform-managed load balancers are preserved)");
            return Ok(());
        }

        println!("  Found {} dynamically created load balancer(s) to delete:", network_lbs.len());
        for lb in &network_lbs {
            println!("    - {} ({}) [status: {}]", lb.name, lb.id, lb.provisioning_status);
        }

        let mut deleted_count = 0;
        let mut failed_count = 0;

        for lb in network_lbs {
            println!("    Deleting load balancer: {} ...", lb.name);

            // Always use cascade delete to handle LB children (listeners, pools, members, monitors)
            let delete_url = format!("{}/lbaas/loadbalancers/{}?cascade=true", self.octavia_endpoint, lb.id);
            match self
                .client
                .delete(&delete_url)
                .header("X-Auth-Token", &self.auth_token)
                .send()
            {
                Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 404 => {
                    // Wait for LB to be deleted (Octavia async deletion)
                    if self.wait_for_lb_deletion(&lb.id, 120).is_ok() {
                        println!("    -> Deleted load balancer: {} (cascade)", lb.name);
                        deleted_count += 1;
                    } else {
                        eprintln!("    WARNING: Load balancer {} deletion timed out (may still be deleting)", lb.name);
                        eprintln!("             Wait a few minutes and retry destroy");
                        failed_count += 1;
                    }
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();
                    eprintln!("    ERROR: Failed to delete {}: {} - {}", lb.name, status, body);
                    failed_count += 1;
                }
                Err(e) => {
                    eprintln!("    ERROR: Failed to delete {}: {}", lb.name, e);
                    failed_count += 1;
                }
            }
        }

        println!("  Load balancers: {} deleted, {} failed", deleted_count, failed_count);

        if failed_count > 0 {
            println!("  WARNING: Some load balancers could not be deleted.");
            println!("           Terraform destroy may still block. You may need to:");
            println!("           1. Wait a few minutes and retry destroy");
            println!("           2. Manually delete LBs from OpenStack dashboard");
        }

        Ok(())
    }

    fn wait_for_lb_deletion(&self, lb_id: &str, timeout_secs: u64) -> Result<()> {
        use std::thread;
        use std::time::{Duration, Instant};

        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("Timeout waiting for LB deletion"));
            }

            let check_url = format!("{}/lbaas/loadbalancers/{}", self.octavia_endpoint, lb_id);
            match self
                .client
                .get(&check_url)
                .header("X-Auth-Token", &self.auth_token)
                .send()
            {
                Ok(resp) if resp.status().as_u16() == 404 => {
                    // LB is deleted
                    return Ok(());
                }
                Ok(resp) if resp.status().is_success() => {
                    // LB still exists, check status
                    if let Ok(lb_check) = resp.json::<serde_json::Value>() {
                        if let Some(status) = lb_check.get("loadbalancer")
                            .and_then(|lb| lb.get("provisioning_status"))
                            .and_then(|s| s.as_str())
                        {
                            if status == "DELETED" || status == "ERROR" {
                                return Ok(());
                            }
                        }
                    }
                    // Still deleting, wait and retry
                    thread::sleep(Duration::from_secs(5));
                }
                _ => {
                    // Error checking status, assume it might be deleted
                    thread::sleep(Duration::from_secs(5));
                }
            }
        }
    }

    fn cleanup_floating_ips(&self) -> Result<()> {
        println!("\nChecking for orphaned floating IPs...");

        let url = format!("{}/floatingips", self.neutron_endpoint);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .context("Failed to list floating IPs")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            eprintln!("  WARNING: Failed to list floating IPs ({}): {}", status, body);
            return Ok(());
        }

        let fips_response: FloatingIPsResponse = response
            .json()
            .context("Failed to parse floating IPs response")?;

        // Find orphaned floating IPs (status DOWN or not associated with a port)
        let orphaned_fips: Vec<&FloatingIP> = fips_response
            .floatingips
            .iter()
            .filter(|fip| fip.status.to_lowercase() == "down" || fip.port_id.is_none())
            .collect();

        if orphaned_fips.is_empty() {
            println!("  -> No orphaned floating IPs found");
            return Ok(());
        }

        println!("  Found {} orphaned floating IP(s):", orphaned_fips.len());
        for fip in &orphaned_fips {
            println!("    - {} ({})", fip.floating_ip_address, fip.id);
        }

        let mut deleted_count = 0;
        let mut failed_count = 0;

        for fip in orphaned_fips {
            let delete_url = format!("{}/floatingips/{}", self.neutron_endpoint, fip.id);
            match self
                .client
                .delete(&delete_url)
                .header("X-Auth-Token", &self.auth_token)
                .send()
            {
                Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 404 => {
                    println!("    -> Deleted floating IP: {}", fip.floating_ip_address);
                    deleted_count += 1;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();
                    eprintln!("    ERROR: Failed to delete {}: {} - {}", fip.floating_ip_address, status, body);
                    failed_count += 1;
                }
                Err(e) => {
                    eprintln!("    ERROR: Failed to delete {}: {}", fip.floating_ip_address, e);
                    failed_count += 1;
                }
            }
        }

        println!("  Floating IPs: {} deleted, {} failed", deleted_count, failed_count);
        Ok(())
    }

    fn cleanup_loadbalancer_ports(&self) -> Result<()> {
        println!("\nChecking for orphaned load balancer ports...");

        let url = format!("{}/ports", self.neutron_endpoint);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .context("Failed to list ports")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            eprintln!("  WARNING: Failed to list ports ({}): {}", status, body);
            return Ok(());
        }

        let ports_response: PortsResponse = response
            .json()
            .context("Failed to parse ports response")?;

        // Find Octavia load balancer ports
        let lb_ports: Vec<&Port> = ports_response
            .ports
            .iter()
            .filter(|p| p.device_owner.starts_with("Octavia") || p.device_owner.starts_with("octavia"))
            .collect();

        if lb_ports.is_empty() {
            println!("  -> No orphaned load balancer ports found");
            return Ok(());
        }

        println!("  Found {} load balancer port(s):", lb_ports.len());
        for port in &lb_ports {
            println!("    - {} ({})", port.name, port.id);
        }

        let mut deleted_count = 0;
        let mut failed_count = 0;

        for port in lb_ports {
            let delete_url = format!("{}/ports/{}", self.neutron_endpoint, port.id);
            match self
                .client
                .delete(&delete_url)
                .header("X-Auth-Token", &self.auth_token)
                .send()
            {
                Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 404 => {
                    println!("    -> Deleted port: {}", port.name);
                    deleted_count += 1;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();
                    eprintln!("    ERROR: Failed to delete {}: {} - {}", port.name, status, body);
                    failed_count += 1;
                }
                Err(e) => {
                    eprintln!("    ERROR: Failed to delete {}: {}", port.name, e);
                    failed_count += 1;
                }
            }
        }

        println!("  Load balancer ports: {} deleted, {} failed", deleted_count, failed_count);
        Ok(())
    }

    fn cleanup_network_ports(&self, network_id: &str) -> Result<()> {
        println!("\nChecking for orphaned network ports on {}...", network_id);

        let url = format!("{}/ports?network_id={}", self.neutron_endpoint, network_id);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .context("Failed to list network ports")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            eprintln!("  WARNING: Failed to list network ports ({}): {}", status, body);
            return Ok(());
        }

        let ports_response: PortsResponse = response
            .json()
            .context("Failed to parse network ports response")?;

        // Find orphaned ports (not owned by compute, router, or DHCP)
        let orphaned_ports: Vec<&Port> = ports_response
            .ports
            .iter()
            .filter(|p| {
                !p.device_owner.starts_with("compute:")
                    && !p.device_owner.starts_with("network:router_")
                    && !p.device_owner.starts_with("network:dhcp")
            })
            .collect();

        if orphaned_ports.is_empty() {
            println!("  -> No orphaned network ports found");
            return Ok(());
        }

        println!("  Found {} orphaned network port(s):", orphaned_ports.len());
        for port in &orphaned_ports {
            println!("    - {} ({}) [{}]", port.name, port.id, port.device_owner);
        }

        let mut deleted_count = 0;
        let mut failed_count = 0;

        for port in orphaned_ports {
            let delete_url = format!("{}/ports/{}", self.neutron_endpoint, port.id);
            match self
                .client
                .delete(&delete_url)
                .header("X-Auth-Token", &self.auth_token)
                .send()
            {
                Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 404 => {
                    println!("    -> Deleted port: {}", port.name);
                    deleted_count += 1;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();
                    eprintln!("    ERROR: Failed to delete {}: {} - {}", port.name, status, body);
                    failed_count += 1;
                }
                Err(e) => {
                    eprintln!("    ERROR: Failed to delete {}: {}", port.name, e);
                    failed_count += 1;
                }
            }
        }

        println!("  Network ports: {} deleted, {} failed", deleted_count, failed_count);
        Ok(())
    }

    fn cleanup_octavia_ports(&self, network_id: &str) -> Result<()> {
        use std::thread;
        use std::time::Duration;

        println!("\nCleaning up Octavia load balancer ports...");

        // Give Octavia a moment to start port cleanup after LB deletion
        thread::sleep(Duration::from_secs(5));

        // First, get the list of all load balancers to identify terraform-managed ones
        let lb_url = format!("{}/lbaas/loadbalancers", self.octavia_endpoint);
        let lb_response = self
            .client
            .get(&lb_url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .context("Failed to list load balancers")?;

        let mut terraform_lb_ids = std::collections::HashSet::new();
        if lb_response.status().is_success() {
            if let Ok(lbs_response) = lb_response.json::<LoadBalancersResponse>() {
                // Identify terraform-managed LBs (ones that end with "-lb")
                for lb in lbs_response.loadbalancers.iter() {
                    if lb.vip_network_id == network_id && lb.name.ends_with("-lb") {
                        terraform_lb_ids.insert(lb.id.clone());
                    }
                }
            }
        }

        let url = format!("{}/ports?network_id={}", self.neutron_endpoint, network_id);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .context("Failed to list network ports")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            eprintln!("  WARNING: Failed to list network ports ({}): {}", status, body);
            return Ok(());
        }

        let ports_response: PortsResponse = response
            .json()
            .context("Failed to parse network ports response")?;

        // Find Octavia ports on this network, excluding terraform-managed ones
        // Port names are typically: octavia-lb-{loadbalancer_id}
        let octavia_ports: Vec<&Port> = ports_response
            .ports
            .iter()
            .filter(|p| {
                let is_octavia = p.device_owner.starts_with("Octavia") || p.device_owner.starts_with("octavia");
                if !is_octavia {
                    return false;
                }

                // Check if this port belongs to a terraform-managed LB
                // Port name format: octavia-lb-{lb_id}
                for tf_lb_id in &terraform_lb_ids {
                    if p.name.contains(tf_lb_id) {
                        return false; // Skip terraform-managed LB ports
                    }
                }

                true
            })
            .collect();

        if octavia_ports.is_empty() {
            println!("  -> No orphaned Octavia ports found on network");
            println!("     (Terraform-managed LB ports are preserved)");
            return Ok(());
        }

        println!("  Found {} orphaned Octavia port(s) to delete:", octavia_ports.len());
        for port in &octavia_ports {
            println!("    - {} ({})", port.name, port.id);
        }

        let mut deleted_count = 0;
        let mut failed_count = 0;

        for port in octavia_ports {
            let delete_url = format!("{}/ports/{}", self.neutron_endpoint, port.id);
            match self
                .client
                .delete(&delete_url)
                .header("X-Auth-Token", &self.auth_token)
                .send()
            {
                Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 404 => {
                    println!("    -> Deleted Octavia port: {}", port.name);
                    deleted_count += 1;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();
                    eprintln!("    ERROR: Failed to delete {}: {} - {}", port.name, status, body);
                    failed_count += 1;
                }
                Err(e) => {
                    eprintln!("    ERROR: Failed to delete {}: {}", port.name, e);
                    failed_count += 1;
                }
            }
        }

        println!("  Octavia ports: {} deleted, {} failed", deleted_count, failed_count);

        if failed_count > 0 {
            eprintln!("  WARNING: Some ports could not be deleted. Terraform destroy may still block.");
            eprintln!("           Wait a moment and retry, or check OpenStack dashboard.");
        }

        Ok(())
    }

    fn cleanup_security_groups(&self, cluster_name: &str) -> Result<()> {
        println!("\nChecking for orphaned security groups...");

        let url = format!("{}/security-groups", self.neutron_endpoint);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .context("Failed to list security groups")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            eprintln!("  WARNING: Failed to list security groups ({}): {}", status, body);
            return Ok(());
        }

        let sgs_response: SecurityGroupsResponse = response
            .json()
            .context("Failed to parse security groups response")?;

        // Find security groups to delete
        let orphaned_sgs: Vec<&SecurityGroup> = sgs_response
            .security_groups
            .iter()
            .filter(|sg| {
                // Match K8s load balancer security groups
                if sg.name.starts_with("lb-sg-") {
                    return true;
                }

                // Also catch any terraform-managed groups that weren't properly deleted
                if sg.name == format!("{}-server", cluster_name)
                    || sg.name == format!("{}-agent", cluster_name) {
                    return true;
                }

                false
            })
            .collect();

        if orphaned_sgs.is_empty() {
            println!("  -> No orphaned security groups found");
            return Ok(());
        }

        println!("  Found {} orphaned security group(s):", orphaned_sgs.len());
        for sg in &orphaned_sgs {
            println!("    - {} ({})", sg.name, sg.id);
        }

        let mut deleted_count = 0;
        let mut failed_count = 0;

        for sg in orphaned_sgs {
            println!("    Deleting security group: {} ...", sg.name);
            let delete_url = format!("{}/security-groups/{}", self.neutron_endpoint, sg.id);
            match self
                .client
                .delete(&delete_url)
                .header("X-Auth-Token", &self.auth_token)
                .send()
            {
                Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 404 => {
                    println!("    -> Deleted security group: {}", sg.name);
                    deleted_count += 1;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();

                    // Security groups might still be in use - this is expected sometimes
                    if status.as_u16() == 409 {
                        eprintln!("    WARNING: Security group {} still in use (will be cleaned up by OpenStack eventually)", sg.name);
                    } else {
                        eprintln!("    ERROR: Failed to delete {}: {} - {}", sg.name, status, body);
                    }
                    failed_count += 1;
                }
                Err(e) => {
                    eprintln!("    ERROR: Failed to delete {}: {}", sg.name, e);
                    failed_count += 1;
                }
            }
        }

        println!("  Security groups: {} deleted, {} failed/skipped", deleted_count, failed_count);

        if failed_count > 0 {
            println!("  Note: Some security groups may still be in use and will be cleaned up automatically by OpenStack");
        }

        Ok(())
    }
}
