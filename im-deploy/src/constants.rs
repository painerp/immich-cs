/// SSH connection constants
pub mod ssh {
    pub const SSH_PORT: u16 = 22;
    pub const SSH_USER: &str = "ubuntu";
    pub const SSH_STRICT_HOST_KEY_CHECKING: &str = "StrictHostKeyChecking=no";
}

/// Network timeouts and retry settings
pub mod network {
    pub const HTTP_TIMEOUT_SECS: u64 = 30;
    pub const RETRY_MAX_ATTEMPTS: u32 = 3;
    pub const RETRY_INITIAL_DELAY_MS: u64 = 1000;
    pub const RETRY_MAX_DELAY_MS: u64 = 30000;
    pub const RETRY_MULTIPLIER: f64 = 2.0;
}

/// OpenStack API constants
pub mod openstack {
    pub const DEFAULT_AUTH_URL: &str = "https://private-cloud.informatik.hs-fulda.de:5000/v3";
    pub const DEFAULT_REGION: &str = "RegionOne";
    pub const DEFAULT_DOMAIN: &str = "Default";
    pub const LOADBALANCER_DELETION_TIMEOUT_SECS: u64 = 120;
    pub const LOADBALANCER_POLL_INTERVAL_SECS: u64 = 5;
}

/// Kubernetes API endpoint constants
pub mod kubernetes {
    pub const API_SERVER_PORT: u16 = 6443;
}

/// Cluster monitoring constants
pub mod monitoring {
    pub const CHECK_INTERVAL_SECS: u64 = 10;
    pub const NODE_READY_TIMEOUT_SECS: u64 = 600;
}

/// Terraform constants
pub mod terraform {
    pub const STATE_DIR: &str = ".terraform";
    pub const TFVARS_FILE: &str = "terraform.tfvars";
    pub const MAIN_TF_FILE: &str = "main.tf";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_constants_values() {
        assert_eq!(ssh::SSH_PORT, 22);
        assert_eq!(ssh::SSH_USER, "ubuntu");
        assert_eq!(ssh::SSH_STRICT_HOST_KEY_CHECKING, "StrictHostKeyChecking=no");
    }

    #[test]
    fn test_network_timeout_values() {
        assert_eq!(network::HTTP_TIMEOUT_SECS, 30);
        assert_eq!(network::RETRY_MAX_ATTEMPTS, 3);
        assert_eq!(network::RETRY_INITIAL_DELAY_MS, 1000);
        assert_eq!(network::RETRY_MAX_DELAY_MS, 30000);
        assert_eq!(network::RETRY_MULTIPLIER, 2.0);
        
        // Verify exponential backoff logic makes sense
        let first_delay = network::RETRY_INITIAL_DELAY_MS;
        let second_delay = (first_delay as f64 * network::RETRY_MULTIPLIER) as u64;
        let third_delay = (second_delay as f64 * network::RETRY_MULTIPLIER) as u64;
        
        assert!(second_delay <= network::RETRY_MAX_DELAY_MS);
        assert_eq!(second_delay, 2000); // 1000 * 2
        assert_eq!(third_delay, 4000); // 2000 * 2
    }

    #[test]
    fn test_openstack_defaults() {
        assert!(openstack::DEFAULT_AUTH_URL.starts_with("https://"));
        assert!(openstack::DEFAULT_AUTH_URL.contains(":5000"));
        assert_eq!(openstack::DEFAULT_REGION, "RegionOne");
        assert_eq!(openstack::DEFAULT_DOMAIN, "Default");
        assert_eq!(openstack::LOADBALANCER_DELETION_TIMEOUT_SECS, 120);
        assert_eq!(openstack::LOADBALANCER_POLL_INTERVAL_SECS, 5);
        
        // Verify timeout is reasonable multiple of poll interval
        assert_eq!(
            openstack::LOADBALANCER_DELETION_TIMEOUT_SECS % openstack::LOADBALANCER_POLL_INTERVAL_SECS,
            0
        );
    }

    #[test]
    fn test_kubernetes_constants() {
        assert_eq!(kubernetes::API_SERVER_PORT, 6443);
    }

    #[test]
    fn test_monitoring_constants() {
        assert_eq!(monitoring::CHECK_INTERVAL_SECS, 10);
        assert_eq!(monitoring::NODE_READY_TIMEOUT_SECS, 600); // 10 minutes
        
        // Verify timeout is reasonable multiple of check interval
        assert_eq!(
            monitoring::NODE_READY_TIMEOUT_SECS % monitoring::CHECK_INTERVAL_SECS,
            0
        );
    }

    #[test]
    fn test_terraform_constants() {
        assert_eq!(terraform::STATE_DIR, ".terraform");
        assert_eq!(terraform::TFVARS_FILE, "terraform.tfvars");
        assert_eq!(terraform::MAIN_TF_FILE, "main.tf");
        
        // Verify file extensions
        assert!(terraform::TFVARS_FILE.ends_with(".tfvars"));
        assert!(terraform::MAIN_TF_FILE.ends_with(".tf"));
    }
}

