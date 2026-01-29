use im_deploy::domain::cluster::{CloudProvider, ServerInfo};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a test ServerInfo with sensible defaults
pub fn create_test_server(name: &str, ip: &str, is_server: bool) -> ServerInfo {
    ServerInfo {
        name: name.to_string(),
        ip: ip.to_string(),
        cloud_provider: "openstack".to_string(),
        tailscale_hostname: if is_server {
            Some(format!("{}.tailnet.ts.net", name))
        } else {
            None
        },
    }
}

/// Creates a test CloudProvider with specified servers
pub fn create_test_provider(
    name: &str,
    bastion_ip: Option<&str>,
    tailscale_enabled: bool,
    servers: Vec<ServerInfo>,
) -> CloudProvider {
    CloudProvider {
        name: name.to_string(),
        bastion_ip: bastion_ip.map(|s| s.to_string()),
        tailscale_enabled,
        servers,
    }
}

/// Creates a temporary directory with terraform structure
/// Returns (TempDir, terraform_dir_path) - keep TempDir alive!
pub fn create_temp_terraform_dir(tfvars_content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let terraform_dir = temp_dir.path().join("terraform");
    fs::create_dir(&terraform_dir).unwrap();

    // Create main.tf
    fs::write(terraform_dir.join("main.tf"), "# test terraform config").unwrap();

    // Create terraform.tfvars
    fs::write(terraform_dir.join("terraform.tfvars"), tfvars_content).unwrap();

    (temp_dir, terraform_dir)
}

/// Returns mock terraform output as JSON string
pub fn mock_terraform_output() -> String {
    include_str!("../fixtures/terraform_outputs.json").to_string()
}

/// Returns mock terraform output without Tailscale
pub fn mock_terraform_output_no_tailscale() -> String {
    include_str!("../fixtures/terraform_outputs_no_tailscale.json").to_string()
}

/// Loads fixture file content
pub fn load_fixture(filename: &str) -> String {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(filename);
    fs::read_to_string(fixture_path).expect(&format!("Failed to load fixture: {}", filename))
}

/// Assert that an error matches a specific variant (helper macro would be better but this works)
pub fn assert_contains(text: &str, pattern: &str) {
    assert!(
        text.contains(pattern),
        "Expected text to contain '{}', but got: {}",
        pattern,
        text
    );
}

