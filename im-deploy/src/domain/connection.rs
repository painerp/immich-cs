use crate::constants::ssh;
use crate::domain::cluster::ServerInfo;
use crate::errors::{Result, SshError};
use std::process::{Command, Stdio};
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub enum ConnectionStrategy {
    Tailscale { hostname: String },
    Bastion { bastion_ip: String, target_ip: String },
}

impl ConnectionStrategy {
    pub fn from_server(server: &ServerInfo, bastion_ip: Option<&str>) -> Result<Self> {
        if let Some(ref hostname) = server.tailscale_hostname {
            Ok(ConnectionStrategy::Tailscale {
                hostname: hostname.clone(),
            })
        } else if let Some(bastion) = bastion_ip {
            Ok(ConnectionStrategy::Bastion {
                bastion_ip: bastion.to_string(),
                target_ip: server.ip.clone(),
            })
        } else {
            Err(SshError::NoConnectionMethod.into())
        }
    }

    pub fn build_ssh_args(&self) -> Vec<String> {
        match self {
            ConnectionStrategy::Tailscale { hostname } => {
                vec![
                    "-o".to_string(),
                    ssh::SSH_STRICT_HOST_KEY_CHECKING.to_string(),
                    format!("{}@{}", ssh::SSH_USER, hostname),
                ]
            }
            ConnectionStrategy::Bastion {
                bastion_ip,
                target_ip,
            } => {
                vec![
                    "-J".to_string(),
                    format!("{}@{}", ssh::SSH_USER, bastion_ip),
                    "-o".to_string(),
                    ssh::SSH_STRICT_HOST_KEY_CHECKING.to_string(),
                    format!("{}@{}", ssh::SSH_USER, target_ip),
                ]
            }
        }
    }

    pub fn execute_interactive(&self) -> Result<()> {
        debug!("Establishing SSH connection: {:?}", self);

        let args = self.build_ssh_args();
        debug!("SSH command: ssh {}", args.join(" "));

        let status = Command::new("ssh")
            .args(&args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;

        if !status.success() {
            return Err(SshError::ConnectionFailed(format!(
                "SSH exited with code {:?}",
                status.code()
            ))
            .into());
        }

        Ok(())
    }

    pub fn execute_command(&self, command: &str) -> Result<std::process::Output> {
        debug!("Executing command over SSH: {}", command);

        let mut args = self.build_ssh_args();
        args.push(command.to_string());

        debug!("SSH command: ssh {}", args.join(" "));

        let output = Command::new("ssh")
            .args(&args)
            .output()
            .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(SshError::CommandFailed {
                command: command.to_string(),
            }
            .into());
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cluster::ServerInfo;

    fn create_test_server(name: &str, ip: &str, tailscale_hostname: Option<&str>) -> ServerInfo {
        ServerInfo {
            name: name.to_string(),
            ip: ip.to_string(),
            cloud_provider: "openstack".to_string(),
            tailscale_hostname: tailscale_hostname.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_connection_strategy_tailscale_builds_correct_args() {
        let strategy = ConnectionStrategy::Tailscale {
            hostname: "server-0.tailnet.ts.net".to_string(),
        };

        let args = strategy.build_ssh_args();

        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "-o");
        assert_eq!(args[1], "StrictHostKeyChecking=no");
        assert_eq!(args[2], "ubuntu@server-0.tailnet.ts.net");
    }

    #[test]
    fn test_connection_strategy_bastion_builds_correct_args() {
        let strategy = ConnectionStrategy::Bastion {
            bastion_ip: "1.2.3.4".to_string(),
            target_ip: "10.0.0.5".to_string(),
        };

        let args = strategy.build_ssh_args();

        assert_eq!(args.len(), 5);
        assert_eq!(args[0], "-J");
        assert_eq!(args[1], "ubuntu@1.2.3.4");
        assert_eq!(args[2], "-o");
        assert_eq!(args[3], "StrictHostKeyChecking=no");
        assert_eq!(args[4], "ubuntu@10.0.0.5");
    }

    #[test]
    fn test_connection_strategy_from_server_prefers_tailscale() {
        let server = create_test_server(
            "k3s-server-0",
            "10.0.0.10",
            Some("server-0.tailnet.ts.net"),
        );

        let strategy = ConnectionStrategy::from_server(&server, Some("1.2.3.4")).unwrap();

        match strategy {
            ConnectionStrategy::Tailscale { hostname } => {
                assert_eq!(hostname, "server-0.tailnet.ts.net");
            }
            _ => panic!("Expected Tailscale strategy"),
        }
    }

    #[test]
    fn test_connection_strategy_from_server_falls_back_to_bastion() {
        let server = create_test_server("k3s-server-0", "10.0.0.10", None);

        let strategy = ConnectionStrategy::from_server(&server, Some("1.2.3.4")).unwrap();

        match strategy {
            ConnectionStrategy::Bastion {
                bastion_ip,
                target_ip,
            } => {
                assert_eq!(bastion_ip, "1.2.3.4");
                assert_eq!(target_ip, "10.0.0.10");
            }
            _ => panic!("Expected Bastion strategy"),
        }
    }

    #[test]
    fn test_connection_strategy_from_server_no_method_errors() {
        let server = create_test_server("k3s-server-0", "10.0.0.10", None);

        let result = ConnectionStrategy::from_server(&server, None);

        assert!(result.is_err());
        let err = result.unwrap_err();
        // SshError::NoConnectionMethod message is "Neither Tailscale nor bastion host available"
        assert!(err.to_string().contains("Neither") || err.to_string().contains("bastion"));
    }

    #[test]
    fn test_connection_strategy_debug_format() {
        let strategy = ConnectionStrategy::Tailscale {
            hostname: "test.ts.net".to_string(),
        };

        let debug_str = format!("{:?}", strategy);
        assert!(debug_str.contains("Tailscale"));
        assert!(debug_str.contains("test.ts.net"));
    }
}


