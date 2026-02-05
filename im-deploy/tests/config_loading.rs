mod common;

use common::{create_temp_terraform_dir, load_fixture};
use im_deploy::config;
use std::env;

#[test]
#[serial_test::serial]
fn test_load_config_with_valid_tfvars() {
    let tfvars = load_fixture("terraform.tfvars");
    let (temp_dir, _terraform_dir) = create_temp_terraform_dir(&tfvars);

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let result = config::load_config(false);

    env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert_eq!(cfg.cluster_name, "test-k3s-cluster");
    assert!(!cfg.dry_run);
    assert!(cfg.tailscale.is_some());
    assert!(cfg.openstack.is_some());

    let ts = cfg.tailscale.unwrap();
    assert_eq!(ts.account_name, "testorg.github");
    assert_eq!(ts.tailnet, "testorg.github.ts.net");

    let os = cfg.openstack.unwrap();
    assert_eq!(os.username, "test-user");
    assert_eq!(os.project_name, "test-project");
    assert_eq!(os.region, "RegionOne");

    drop(temp_dir); // Keep alive until end
}

#[test]
#[serial_test::serial]
fn test_load_config_with_minimal_tfvars() {
    let tfvars = load_fixture("minimal_terraform.tfvars");
    let (temp_dir, _) = create_temp_terraform_dir(&tfvars);
    
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    
    let result = config::load_config(false);
    
    env::set_current_dir(original_dir).unwrap();
    
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert_eq!(cfg.cluster_name, "minimal-cluster");
    assert!(cfg.tailscale.is_none()); // Tailscale not enabled
    assert!(cfg.openstack.is_some());
    
    drop(temp_dir);
}

#[test]
#[serial_test::serial]
fn test_load_config_missing_required_fields() {
    let invalid_tfvars = "cluster_name = \"test\"\n# missing required fields";
    let (temp_dir, _) = create_temp_terraform_dir(invalid_tfvars);
    
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    
    let result = config::load_config(false);
    
    env::set_current_dir(original_dir).unwrap();
    
    // Should succeed but have no OpenStack config
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert!(cfg.openstack.is_none());
    
    drop(temp_dir);
}

#[test]
#[serial_test::serial]
fn test_load_config_with_tailscale_enabled() {
    let tfvars = r#"
cluster_name = "ts-enabled"
user_name = "user"
user_password = "pass"
tenant_name = "project"
enable_tailscale = true
tailscale_api_key = "tskey-test"
tailscale_tailnet = "myorg.tailscale.ts.net"
"#;
    let (temp_dir, _) = create_temp_terraform_dir(tfvars);
    
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    
    let result = config::load_config(false);
    
    env::set_current_dir(original_dir).unwrap();
    
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert!(cfg.tailscale.is_some());
    
    let ts = cfg.tailscale.unwrap();
    assert_eq!(ts.account_name, "myorg.tailscale");
    
    drop(temp_dir);
}

#[test]
#[serial_test::serial]
fn test_load_config_with_openstack_defaults() {
    let tfvars = r#"
cluster_name = "default-test"
user_name = "admin"
user_password = "secret"
tenant_name = "admin-project"
"#;
    let (temp_dir, _) = create_temp_terraform_dir(tfvars);
    
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    
    let result = config::load_config(false);
    
    env::set_current_dir(original_dir).unwrap();
    
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert!(cfg.openstack.is_some());
    
    let os = cfg.openstack.unwrap();
    // Should use default values
    assert!(os.auth_url.contains("private-cloud.informatik.hs-fulda.de"));
    assert_eq!(os.region, "RegionOne");
    assert_eq!(os.insecure, true);
    
    drop(temp_dir);
}

#[test]
#[serial_test::serial]
fn test_load_config_dry_run_mode() {
    let tfvars = load_fixture("minimal_terraform.tfvars");
    let (temp_dir, _) = create_temp_terraform_dir(&tfvars);
    
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    
    let result = config::load_config(true);
    
    env::set_current_dir(original_dir).unwrap();
    
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert!(cfg.dry_run);
    
    drop(temp_dir);
}

#[test]
#[serial_test::serial]
fn test_load_config_invalid_toml_format() {
    let invalid_tfvars = "this is not valid TOML {{{ ]][";
    let (temp_dir, _) = create_temp_terraform_dir(invalid_tfvars);
    
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    
    let result = config::load_config(false);
    
    env::set_current_dir(original_dir).unwrap();
    
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to parse terraform.tfvars"));
    
    drop(temp_dir);
}

#[test]
#[serial_test::serial]
fn test_detect_terraform_dir_not_in_project() {
    // Create temp dir without terraform directory
    let temp_dir = tempfile::TempDir::new().unwrap();

    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let result = config::detect_terraform_dir();

    env::set_current_dir(original_dir).unwrap();

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Terraform directory not found"));
}

