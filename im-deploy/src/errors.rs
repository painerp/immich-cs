use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImDeployError {
    #[error("Terraform error: {0}")]
    Terraform(#[from] TerraformError),

    #[error("OpenStack error: {0}")]
    OpenStack(#[from] OpenStackError),

    #[error("Tailscale error: {0}")]
    Tailscale(#[from] TailscaleError),

    #[error("SSH error: {0}")]
    Ssh(#[from] SshError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum TerraformError {
    #[error("Terraform initialization failed: {0}")]
    InitFailed(String),

    #[error("Terraform command failed: {command}{}", code.map(|c| format!(" (exit code: {})", c)).unwrap_or_default())]
    CommandFailed { command: String, code: Option<i32> },

    #[error("Failed to parse terraform outputs: {0}")]
    OutputParseFailed(String),

    #[error("Terraform directory not found: {0}")]
    DirectoryNotFound(PathBuf),

    #[error("Terraform binary not found. Install terraform or tofu")]
    BinaryNotFound,

    #[error("Failed to extract {resource} from terraform outputs")]
    ResourceNotFound { resource: String },
}

#[derive(Error, Debug)]
pub enum OpenStackError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Failed to list {resource}: {message}")]
    ListFailed { resource: String, message: String },

    #[error("Failed to delete {resource} {id}: {message}")]
    DeleteFailed {
        resource: String,
        id: String,
        message: String,
    },

    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("Resource cleanup timeout: {resource}")]
    CleanupTimeout { resource: String },
}

#[derive(Error, Debug)]
pub enum TailscaleError {
    #[error("Tailscale API request failed: {0}")]
    ApiError(String),

    #[error("Tailscale CLI not installed")]
    CliNotInstalled,

    #[error("Tailscale is not running (state: {0})")]
    NotRunning(String),

    #[error("Connected to wrong Tailscale account. Expected: {expected}, got: {actual}")]
    WrongAccount { expected: String, actual: String },

    #[error("Failed to switch Tailscale account")]
    AccountSwitchFailed,

    #[error("Failed to parse Tailscale response: {0}")]
    ParseError(String),
}

#[derive(Error, Debug)]
pub enum SshError {
    #[error("SSH connection failed: {0}")]
    ConnectionFailed(String),

    #[error("SSH command execution failed: {command}")]
    CommandFailed { command: String },

    #[error("Neither Tailscale nor bastion host available for connection")]
    NoConnectionMethod,

    #[error("Tailscale hostname not found for server {0}")]
    TailscaleHostnameNotFound(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Terraform directory not found. Run from project root or im-deploy directory")]
    TerraformDirNotFound,

    #[error("Failed to parse terraform.tfvars: {0}")]
    TfVarsParseFailed(String),

    #[error("Missing required configuration field: {0}")]
    MissingField(String),

    #[error("Invalid configuration value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },
}

pub type Result<T> = std::result::Result<T, ImDeployError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terraform_error_display_messages() {
        let err = TerraformError::InitFailed("connection timeout".to_string());
        assert!(err.to_string().contains("Terraform initialization failed"));
        assert!(err.to_string().contains("connection timeout"));

        let err = TerraformError::CommandFailed {
            command: "terraform apply".to_string(),
            code: Some(1),
        };
        assert!(err.to_string().contains("terraform apply"));
        assert!(err.to_string().contains("exit code: 1"));

        let err = TerraformError::CommandFailed {
            command: "terraform plan".to_string(),
            code: None,
        };
        assert!(err.to_string().contains("terraform plan"));
        assert!(!err.to_string().contains("exit code"));

        let err = TerraformError::BinaryNotFound;
        assert!(err.to_string().contains("Install terraform or tofu"));

        let err = TerraformError::ResourceNotFound {
            resource: "load balancer IP".to_string(),
        };
        assert!(err.to_string().contains("load balancer IP"));
    }

    #[test]
    fn test_error_conversion_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let deploy_err: ImDeployError = io_err.into();

        match deploy_err {
            ImDeployError::Io(_) => {} // Success
            _ => panic!("Expected Io error variant"),
        }
    }

    #[test]
    fn test_error_conversion_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("generic error");
        let deploy_err: ImDeployError = anyhow_err.into();

        match deploy_err {
            ImDeployError::Other(_) => {} // Success
            _ => panic!("Expected Other error variant"),
        }
    }

    #[test]
    fn test_ssh_error_variants() {
        let err = SshError::ConnectionFailed("timeout".to_string());
        assert!(err.to_string().contains("SSH connection failed"));
        assert!(err.to_string().contains("timeout"));

        let err = SshError::NoConnectionMethod;
        assert!(err
            .to_string()
            .contains("Neither Tailscale nor bastion host"));

        let err = SshError::TailscaleHostnameNotFound("k3s-server-0".to_string());
        assert!(err.to_string().contains("Tailscale hostname not found"));
        assert!(err.to_string().contains("k3s-server-0"));

        let err = SshError::CommandFailed {
            command: "kubectl get nodes".to_string(),
        };
        assert!(err.to_string().contains("kubectl get nodes"));
    }

    #[test]
    fn test_config_error_variants() {
        let err = ConfigError::TerraformDirNotFound;
        assert!(err.to_string().contains("Terraform directory not found"));

        let err = ConfigError::MissingField("tailscale_api_key".to_string());
        assert!(err.to_string().contains("Missing required configuration"));
        assert!(err.to_string().contains("tailscale_api_key"));

        let err = ConfigError::InvalidValue {
            field: "port".to_string(),
            reason: "must be between 1-65535".to_string(),
        };
        assert!(err.to_string().contains("Invalid configuration value"));
        assert!(err.to_string().contains("port"));
        assert!(err.to_string().contains("1-65535"));
    }

    #[test]
    fn test_openstack_error_variants() {
        let err = OpenStackError::AuthFailed("invalid credentials".to_string());
        assert!(err.to_string().contains("Authentication failed"));

        let err = OpenStackError::DeleteFailed {
            resource: "load balancer".to_string(),
            id: "lb-123".to_string(),
            message: "still in use".to_string(),
        };
        assert!(err.to_string().contains("load balancer"));
        assert!(err.to_string().contains("lb-123"));
        assert!(err.to_string().contains("still in use"));
    }

    #[test]
    fn test_tailscale_error_variants() {
        let err = TailscaleError::CliNotInstalled;
        assert!(err.to_string().contains("not installed"));

        let err = TailscaleError::NotRunning("Stopped".to_string());
        assert!(err.to_string().contains("not running"));
        assert!(err.to_string().contains("Stopped"));

        let err = TailscaleError::WrongAccount {
            expected: "org1.github".to_string(),
            actual: "org2.github".to_string(),
        };
        // Check for "Connected to wrong" instead of just "Wrong"
        assert!(err.to_string().contains("wrong") || err.to_string().contains("Expected"));
        assert!(err.to_string().contains("org1.github"));
        assert!(err.to_string().contains("org2.github"));
    }
}


