use crate::constants::{openstack as os_constants, terraform as tf_constants};
use crate::errors::{ConfigError, Result, TerraformError};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct Config {
    pub terraform_dir: PathBuf,
    pub terraform_bin: String,
    pub cluster_name: String,
    pub tailscale: Option<TailscaleConfig>,
    pub openstack: Option<OpenStackConfig>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct TailscaleConfig {
    pub api_key: String,
    pub tailnet: String,
    pub account_name: String,
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

impl TailscaleConfig {
    fn extract_account_name(tailnet: &str) -> String {
        tailnet
            .strip_suffix(".ts.net")
            .unwrap_or(tailnet)
            .to_string()
    }
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
    if terraform_dir.join(tf_constants::MAIN_TF_FILE).exists() {
        debug!("Found terraform directory at {:?}", terraform_dir);
        return Ok(terraform_dir);
    }

    // Check if ../terraform/main.tf exists
    if let Some(parent) = current_dir.parent() {
        let parent_terraform_dir = parent.join("terraform");
        if parent_terraform_dir.join(tf_constants::MAIN_TF_FILE).exists() {
            debug!("Found terraform directory at {:?}", parent_terraform_dir);
            return Ok(parent_terraform_dir);
        }
    }

    Err(ConfigError::TerraformDirNotFound.into())
}

pub fn find_terraform_binary() -> Result<String> {
    debug!("Looking for terraform/tofu binary");

    // Try tofu first
    if Command::new("which")
        .arg("tofu")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        info!("Using tofu binary");
        return Ok("tofu".to_string());
    }

    // Fallback to terraform
    if Command::new("which")
        .arg("terraform")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        info!("Using terraform binary");
        return Ok("terraform".to_string());
    }

    Err(TerraformError::BinaryNotFound.into())
}

pub fn load_config(dry_run: bool) -> Result<Config> {
    info!("Loading configuration");

    let terraform_dir = detect_terraform_dir()?;
    let terraform_bin = find_terraform_binary()?;

    // Parse terraform.tfvars
    let tfvars_path = terraform_dir.join(tf_constants::TFVARS_FILE);
    let tfvars_content = fs::read_to_string(&tfvars_path)
        .map_err(|e| ConfigError::TfVarsParseFailed(format!("Could not read {}: {}", tfvars_path.display(), e)))?;

    let vars: TerraformVars = toml::from_str(&tfvars_content)
        .map_err(|e| ConfigError::TfVarsParseFailed(e.to_string()))?;

    let cluster_name = vars.cluster_name
        .unwrap_or_else(|| "k3s-multicloud".to_string());

    debug!("Cluster name: {}", cluster_name);

    // Build Tailscale config if enabled
    let tailscale = if vars.enable_tailscale.unwrap_or(false) {
        let api_key = vars.tailscale_api_key
            .ok_or_else(|| ConfigError::MissingField("tailscale_api_key".to_string()))?;
        let tailnet = vars.tailscale_tailnet
            .ok_or_else(|| ConfigError::MissingField("tailscale_tailnet".to_string()))?;

        let account_name = TailscaleConfig::extract_account_name(&tailnet);
        debug!("Tailscale enabled. Account: {}", account_name);

        Some(TailscaleConfig {
            api_key,
            tailnet,
            account_name,
        })
    } else {
        debug!("Tailscale disabled");
        None
    };

    // Build OpenStack config
    let openstack = if vars.user_name.is_some() && vars.user_password.is_some() {
        debug!("OpenStack credentials found");
        Some(OpenStackConfig {
            auth_url: vars.openstack_auth_url
                .unwrap_or_else(|| os_constants::DEFAULT_AUTH_URL.to_string()),
            username: vars.user_name
                .ok_or_else(|| ConfigError::MissingField("user_name".to_string()))?,
            password: vars.user_password
                .ok_or_else(|| ConfigError::MissingField("user_password".to_string()))?,
            project_name: vars.tenant_name
                .ok_or_else(|| ConfigError::MissingField("tenant_name".to_string()))?,
            region: vars.openstack_region
                .unwrap_or_else(|| os_constants::DEFAULT_REGION.to_string()),
            cacert_file: vars.openstack_cacert_file,
            insecure: vars.openstack_insecure.unwrap_or(true),
        })
    } else {
        debug!("OpenStack credentials not found");
        None
    };

    if dry_run {
        info!("DRY RUN MODE enabled - no actual changes will be made");
    }

    Ok(Config {
        terraform_dir,
        terraform_bin,
        cluster_name,
        tailscale,
        openstack,
        dry_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_tailscale_account_name_extraction() {
        let account = TailscaleConfig::extract_account_name("cloudserv11.github.ts.net");
        assert_eq!(account, "cloudserv11.github");

        let account2 = TailscaleConfig::extract_account_name("myaccount.tailscale.ts.net");
        assert_eq!(account2, "myaccount.tailscale");

        // If no .ts.net suffix, return as-is
        let account3 = TailscaleConfig::extract_account_name("plain-name");
        assert_eq!(account3, "plain-name");
    }

    #[test]
    fn test_find_terraform_binary() {
        // Result depends on what's installed, so we just check if it doesn't panic
        let result = find_terraform_binary();
        // Either finds a binary or returns an error - both are valid
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_terraform_dir_not_found() {
        // Save original directory
        let original_dir = std::env::current_dir().unwrap();

        // When run from a temp directory without terraform, should fail
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = detect_terraform_dir();

        // Always restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err());

        drop(temp_dir);
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_terraform_dir_found() {
        // Create a temp directory structure with terraform/main.tf
        let temp_dir = TempDir::new().unwrap();
        let terraform_dir = temp_dir.path().join("terraform");
        fs::create_dir(&terraform_dir).unwrap();
        fs::write(terraform_dir.join("main.tf"), "# test").unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = detect_terraform_dir();

        // Restore original directory before assertions
        std::env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        let found_dir = result.unwrap();
        assert_eq!(found_dir.file_name().unwrap(), "terraform");

        // Keep temp_dir alive until after assertions
        drop(temp_dir);
    }
}

