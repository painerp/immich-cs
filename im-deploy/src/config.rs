use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Config {
    pub terraform_dir: PathBuf,
    pub terraform_bin: String,
    pub cluster_name: String,
    pub tailscale: Option<TailscaleConfig>,
    pub openstack: Option<OpenStackConfig>,
}

#[derive(Debug, Clone)]
pub struct TailscaleConfig {
    pub api_key: String,
    pub tailnet: String,
}

#[derive(Debug, Clone)]
pub struct OpenStackConfig {
    pub auth_url: String,
    pub username: String,
    pub password: String,
    pub project_name: String,
    pub region: String,
    pub cacert_file: Option<String>,
    pub insecure: bool,
}

#[derive(Debug, Deserialize)]
struct TerraformVars {
    cluster_name: Option<String>,
    user_name: Option<String>,
    user_password: Option<String>,
    tenant_name: Option<String>,
    openstack_auth_url: Option<String>,
    openstack_region: Option<String>,
    openstack_cacert_file: Option<String>,
    openstack_insecure: Option<bool>,
    enable_tailscale: Option<bool>,
    tailscale_api_key: Option<String>,
    tailscale_tailnet: Option<String>,
}

pub fn detect_terraform_dir() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;

    // Check if ./terraform/main.tf exists
    let terraform_dir = current_dir.join("terraform");
    if terraform_dir.join("main.tf").exists() {
        return Ok(terraform_dir);
    }

    // Check if ../terraform/main.tf exists
    if let Some(parent) = current_dir.parent() {
        let parent_terraform_dir = parent.join("terraform");
        if parent_terraform_dir.join("main.tf").exists() {
            return Ok(parent_terraform_dir);
        }
    }

    bail!("Could not find terraform directory. Please run from project root or im-deploy directory.");
}

pub fn find_terraform_binary() -> Result<String> {
    // Try tofu first
    if Command::new("which")
        .arg("tofu")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Ok("tofu".to_string());
    }

    // Fallback to terraform
    if Command::new("which")
        .arg("terraform")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Ok("terraform".to_string());
    }

    bail!("Neither 'tofu' nor 'terraform' binary found. Please install one of them.");
}

pub fn load_config() -> Result<Config> {
    let terraform_dir = detect_terraform_dir()?;
    let terraform_bin = find_terraform_binary()?;

    // Parse terraform.tfvars
    let tfvars_path = terraform_dir.join("terraform.tfvars");
    let tfvars_content = fs::read_to_string(&tfvars_path)
        .with_context(|| format!("Failed to read {}", tfvars_path.display()))?;

    let vars: TerraformVars = toml::from_str(&tfvars_content)
        .context("Failed to parse terraform.tfvars as TOML")?;

    let cluster_name = vars.cluster_name
        .unwrap_or_else(|| "k3s-multicloud".to_string());

    // Build Tailscale config if enabled
    let tailscale = if vars.enable_tailscale.unwrap_or(false) {
        Some(TailscaleConfig {
            api_key: vars.tailscale_api_key
                .context("tailscale_api_key not found in terraform.tfvars")?,
            tailnet: vars.tailscale_tailnet
                .context("tailscale_tailnet not found in terraform.tfvars")?,
        })
    } else {
        None
    };

    // Build OpenStack config
    let openstack = if vars.user_name.is_some() && vars.user_password.is_some() {
        Some(OpenStackConfig {
            auth_url: vars.openstack_auth_url
                .unwrap_or_else(|| "https://private-cloud.informatik.hs-fulda.de:5000/v3".to_string()),
            username: vars.user_name
                .context("user_name not found in terraform.tfvars")?,
            password: vars.user_password
                .context("user_password not found in terraform.tfvars")?,
            project_name: vars.tenant_name
                .context("tenant_name not found in terraform.tfvars")?,
            region: vars.openstack_region
                .unwrap_or_else(|| "RegionOne".to_string()),
            cacert_file: vars.openstack_cacert_file,
            insecure: vars.openstack_insecure.unwrap_or(true),
        })
    } else {
        None
    };

    Ok(Config {
        terraform_dir,
        terraform_bin,
        cluster_name,
        tailscale,
        openstack,
    })
}

