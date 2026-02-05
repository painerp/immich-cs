mod common;

use common::{load_fixture, mock_terraform_output, mock_terraform_output_no_tailscale};
use serde_json::Value;

#[test]
fn test_parse_valid_terraform_output() {
    let output_json = mock_terraform_output();
    let output: Value = serde_json::from_str(&output_json).unwrap();

    // Verify structure
    assert!(output.get("openstack_cluster").is_some());
    assert!(output.get("tailscale_enabled").is_some());

    // Verify OpenStack cluster values
    let cluster = output["openstack_cluster"]["value"].as_object().unwrap();
    assert_eq!(cluster["cluster_name"], "test-cluster");
    assert_eq!(cluster["network_id"], "net-12345");

    // Verify server IPs
    let server_ips = cluster["server_ips"].as_array().unwrap();
    assert_eq!(server_ips.len(), 3);
    assert_eq!(server_ips[0], "10.0.1.10");

    // Verify agent IPs
    let agent_ips = cluster["agent_ips"].as_array().unwrap();
    assert_eq!(agent_ips.len(), 2);
}

#[test]
fn test_parse_terraform_output_with_tailscale() {
    let output_json = mock_terraform_output();
    let output: Value = serde_json::from_str(&output_json).unwrap();

    let tailscale_enabled = output["tailscale_enabled"]["value"].as_bool().unwrap();
    assert!(tailscale_enabled);

    let hostnames = output["tailscale_hostnames"]["value"].as_object().unwrap();
    let openstack_servers = hostnames["openstack_servers"].as_array().unwrap();
    assert_eq!(openstack_servers.len(), 3);
    assert_eq!(openstack_servers[0], "k3s-server-0.tailnet.ts.net");
}

#[test]
fn test_parse_terraform_output_without_tailscale() {
    let output_json = mock_terraform_output_no_tailscale();
    let output: Value = serde_json::from_str(&output_json).unwrap();

    let tailscale_enabled = output["tailscale_enabled"]["value"].as_bool().unwrap();
    assert!(!tailscale_enabled);

    // Should have bastion IP
    let cluster = output["openstack_cluster"]["value"].as_object().unwrap();
    assert!(cluster.get("bastion_ip").is_some());
}

#[test]
fn test_parse_terraform_output_gpu_enabled() {
    let output_json = mock_terraform_output();
    let output: Value = serde_json::from_str(&output_json).unwrap();

    let gpu_enabled = output["enable_nvidia_gpu_operator"]["value"].as_bool().unwrap();
    assert!(gpu_enabled);
}

#[test]
fn test_parse_terraform_output_argocd_enabled() {
    let output_json = mock_terraform_output();
    let output: Value = serde_json::from_str(&output_json).unwrap();

    let argocd_enabled = output["enable_argocd"]["value"].as_bool().unwrap();
    assert!(argocd_enabled);
}

#[test]
fn test_parse_load_balancer_ip_extraction() {
    let output_json = mock_terraform_output();
    let output: Value = serde_json::from_str(&output_json).unwrap();

    // From primary_api_endpoint
    let endpoint = output["primary_api_endpoint"]["value"].as_str().unwrap();
    assert_eq!(endpoint, "https://5.6.7.8:6443");

    let lb_ip = endpoint
        .trim_start_matches("https://")
        .trim_end_matches(":6443");
    assert_eq!(lb_ip, "5.6.7.8");

    // Also from cluster loadbalancer_ip
    let cluster = output["openstack_cluster"]["value"].as_object().unwrap();
    let direct_lb_ip = cluster["loadbalancer_ip"].as_str().unwrap();
    assert_eq!(direct_lb_ip, "5.6.7.8");
}

#[test]
fn test_parse_malformed_json() {
    let malformed = "{invalid json";
    let result: Result<Value, _> = serde_json::from_str(malformed);
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_output() {
    let empty_output = "{}";
    let output: Value = serde_json::from_str(empty_output).unwrap();

    // Should parse but have no fields
    assert!(output.get("openstack_cluster").is_none());
    assert!(output.get("tailscale_enabled").is_none());
}

#[test]
fn test_count_servers_and_agents() {
    let output_json = mock_terraform_output();
    let output: Value = serde_json::from_str(&output_json).unwrap();

    let all_server_ips = output["all_server_ips"]["value"].as_array().unwrap();
    let all_agent_ips = output["all_agent_ips"]["value"].as_array().unwrap();

    assert_eq!(all_server_ips.len(), 3);
    assert_eq!(all_agent_ips.len(), 2);

    let total_nodes = all_server_ips.len() + all_agent_ips.len();
    assert_eq!(total_nodes, 5);
}

