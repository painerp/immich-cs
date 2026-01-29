use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub ip: String,
    pub cloud_provider: String,
    pub tailscale_hostname: Option<String>,
}

impl ServerInfo {
    pub fn is_server(&self) -> bool {
        self.name.contains("server")
    }

    pub fn is_agent(&self) -> bool {
        self.name.contains("agent")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProvider {
    pub name: String,
    pub bastion_ip: Option<String>,
    pub tailscale_enabled: bool,
    pub servers: Vec<ServerInfo>,
}

impl CloudProvider {
    pub fn server_count(&self) -> usize {
        self.servers.iter().filter(|s| s.is_server()).count()
    }

    pub fn agent_count(&self) -> usize {
        self.servers.iter().filter(|s| s.is_agent()).count()
    }

    pub fn total_nodes(&self) -> usize {
        self.servers.len()
    }

    pub fn get_first_server(&self) -> Option<&ServerInfo> {
        self.servers.iter().find(|s| s.is_server())
    }
}

#[derive(Debug, Clone)]
pub struct ClusterInfo {
    pub cluster_name: String,
    pub providers: Vec<CloudProvider>,
    pub primary_api_endpoint: Option<String>,
    pub gpu_enabled: bool,
    pub argocd_enabled: bool,
}

impl ClusterInfo {
    pub fn total_expected_nodes(&self) -> usize {
        self.providers.iter().map(|p| p.total_nodes()).sum()
    }

    pub fn primary_provider(&self) -> Option<&CloudProvider> {
        self.providers.first()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_info_is_server() {
        let server = ServerInfo {
            name: "k3s-server-0".to_string(),
            ip: "10.0.0.1".to_string(),
            cloud_provider: "openstack".to_string(),
            tailscale_hostname: None,
        };
        assert!(server.is_server());
        assert!(!server.is_agent());
    }

    #[test]
    fn test_server_info_is_agent() {
        let agent = ServerInfo {
            name: "k3s-agent-0".to_string(),
            ip: "10.0.0.2".to_string(),
            cloud_provider: "openstack".to_string(),
            tailscale_hostname: None,
        };
        assert!(!agent.is_server());
        assert!(agent.is_agent());
    }

    #[test]
    fn test_cloud_provider_counts() {
        let provider = CloudProvider {
            name: "OpenStack".to_string(),
            bastion_ip: Some("1.2.3.4".to_string()),
            tailscale_enabled: false,
            servers: vec![
                ServerInfo {
                    name: "k3s-server-0".to_string(),
                    ip: "10.0.0.1".to_string(),
                    cloud_provider: "openstack".to_string(),
                    tailscale_hostname: None,
                },
                ServerInfo {
                    name: "k3s-agent-0".to_string(),
                    ip: "10.0.0.2".to_string(),
                    cloud_provider: "openstack".to_string(),
                    tailscale_hostname: None,
                },
                ServerInfo {
                    name: "k3s-agent-1".to_string(),
                    ip: "10.0.0.3".to_string(),
                    cloud_provider: "openstack".to_string(),
                    tailscale_hostname: None,
                },
            ],
        };

        assert_eq!(provider.server_count(), 1);
        assert_eq!(provider.agent_count(), 2);
        assert_eq!(provider.total_nodes(), 3);
    }

    #[test]
    fn test_cloud_provider_get_first_server() {
        let provider = CloudProvider {
            name: "OpenStack".to_string(),
            bastion_ip: None,
            tailscale_enabled: true,
            servers: vec![
                ServerInfo {
                    name: "k3s-agent-0".to_string(),
                    ip: "10.0.0.2".to_string(),
                    cloud_provider: "openstack".to_string(),
                    tailscale_hostname: None,
                },
                ServerInfo {
                    name: "k3s-server-0".to_string(),
                    ip: "10.0.0.1".to_string(),
                    cloud_provider: "openstack".to_string(),
                    tailscale_hostname: Some("server-0.tailscale.net".to_string()),
                },
            ],
        };

        let first_server = provider.get_first_server();
        assert!(first_server.is_some());
        assert_eq!(first_server.unwrap().name, "k3s-server-0");
    }

    #[test]
    fn test_cluster_info_total_nodes_multiple_providers() {
        let cluster_info = ClusterInfo {
            cluster_name: "multi-cloud".to_string(),
            providers: vec![
                CloudProvider {
                    name: "OpenStack".to_string(),
                    bastion_ip: None,
                    tailscale_enabled: true,
                    servers: vec![
                        ServerInfo {
                            name: "k3s-server-0".to_string(),
                            ip: "10.0.0.1".to_string(),
                            cloud_provider: "openstack".to_string(),
                            tailscale_hostname: None,
                        },
                        ServerInfo {
                            name: "k3s-agent-0".to_string(),
                            ip: "10.0.0.2".to_string(),
                            cloud_provider: "openstack".to_string(),
                            tailscale_hostname: None,
                        },
                    ],
                },
                CloudProvider {
                    name: "AWS".to_string(),
                    bastion_ip: Some("1.2.3.4".to_string()),
                    tailscale_enabled: false,
                    servers: vec![ServerInfo {
                        name: "k3s-agent-1".to_string(),
                        ip: "172.16.0.1".to_string(),
                        cloud_provider: "aws".to_string(),
                        tailscale_hostname: None,
                    }],
                },
            ],
            primary_api_endpoint: None,
            gpu_enabled: false,
            argocd_enabled: false,
        };

        assert_eq!(cluster_info.total_expected_nodes(), 3);
    }

    #[test]
    fn test_cluster_info_primary_provider_returns_first() {
        let cluster_info = ClusterInfo {
            cluster_name: "test".to_string(),
            providers: vec![
                CloudProvider {
                    name: "Provider1".to_string(),
                    bastion_ip: None,
                    tailscale_enabled: false,
                    servers: vec![],
                },
                CloudProvider {
                    name: "Provider2".to_string(),
                    bastion_ip: None,
                    tailscale_enabled: false,
                    servers: vec![],
                },
            ],
            primary_api_endpoint: None,
            gpu_enabled: false,
            argocd_enabled: false,
        };

        let primary = cluster_info.primary_provider();
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().name, "Provider1");
    }

    #[test]
    fn test_cloud_provider_empty_servers() {
        let provider = CloudProvider {
            name: "Empty".to_string(),
            bastion_ip: None,
            tailscale_enabled: false,
            servers: vec![],
        };

        assert_eq!(provider.server_count(), 0);
        assert_eq!(provider.agent_count(), 0);
        assert_eq!(provider.total_nodes(), 0);
        assert!(provider.get_first_server().is_none());
    }

    #[test]
    fn test_server_info_serialization() {
        let server = ServerInfo {
            name: "test-server".to_string(),
            ip: "192.168.1.1".to_string(),
            cloud_provider: "test-cloud".to_string(),
            tailscale_hostname: Some("test.ts.net".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&server).unwrap();
        assert!(json.contains("test-server"));
        assert!(json.contains("192.168.1.1"));
        assert!(json.contains("test.ts.net"));

        // Deserialize back
        let deserialized: ServerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, server.name);
        assert_eq!(deserialized.ip, server.ip);
        assert_eq!(deserialized.tailscale_hostname, server.tailscale_hostname);
    }
}


