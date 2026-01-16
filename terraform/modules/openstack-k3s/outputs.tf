# Module Outputs
output "cluster_name" {
  description = "Name of the K3s cluster"
  value       = var.cluster_name
}
output "bastion_ip" {
  description = "Public IP address of the bastion host"
  value       = var.enable_bastion ? openstack_networking_floatingip_v2.fip_bastion[0].address : null
}
output "loadbalancer_ip" {
  description = "Public IP address of the load balancer"
  value       = var.enable_load_balancer ? openstack_networking_floatingip_v2.fip_lb[0].address : null
}
output "loadbalancer_internal_vip" {
  description = "Internal VIP address of the load balancer"
  value       = var.enable_load_balancer ? openstack_lb_loadbalancer_v2.k3s_lb[0].vip_address : null
}
output "server_ips" {
  description = "Private IP addresses of server nodes"
  value       = openstack_compute_instance_v2.k3s_server[*].access_ip_v4
}
output "server_ids" {
  description = "Instance IDs of server nodes"
  value       = openstack_compute_instance_v2.k3s_server[*].id
}
output "agent_ips" {
  description = "Private IP addresses of agent nodes"
  value       = openstack_compute_instance_v2.k3s_agent[*].access_ip_v4
}
output "agent_ids" {
  description = "Instance IDs of agent nodes"
  value       = openstack_compute_instance_v2.k3s_agent[*].id
}
output "network_id" {
  description = "ID of the created network"
  value       = openstack_networking_network_v2.network.id
}
output "subnet_id" {
  description = "ID of the created subnet"
  value       = openstack_networking_subnet_v2.subnet.id
}
output "security_group_ids" {
  description = "Map of security group IDs"
  value = {
    server  = openstack_networking_secgroup_v2.server.id
    agent   = openstack_networking_secgroup_v2.agent.id
    bastion = var.enable_bastion ? openstack_networking_secgroup_v2.bastion[0].id : null
  }
}
output "kubeconfig_command" {
  description = "Command to fetch kubeconfig from the first server"
  value       = var.enable_bastion ? "ssh -J ubuntu@${openstack_networking_floatingip_v2.fip_bastion[0].address} ubuntu@${openstack_compute_instance_v2.k3s_server[0].access_ip_v4} 'sudo cat /etc/rancher/k3s/k3s.yaml'" : null
}

###############################################################################
# Tailscale Outputs
###############################################################################

output "tailscale_enabled" {
  description = "Whether Tailscale is enabled on this cluster"
  value       = var.enable_tailscale
}

output "tailscale_keys_auto_generated" {
  description = "Whether Tailscale auth keys were auto-generated (ephemeral mode)"
  value       = var.enable_tailscale
}

output "tailscale_key_descriptions" {
  description = "Descriptions of generated Tailscale auth keys for tracking in admin console"
  value = var.enable_tailscale ? concat(
    [for i in range(var.server_count) : "k3s-${var.cluster_name}-server-${i}"],
    [for i in range(var.agent_count) : "k3s-${var.cluster_name}-agent-${i}"]
  ) : []
}

output "tailscale_server_hostnames" {
  description = "Tailscale MagicDNS hostnames for server nodes"
  value = var.enable_tailscale ? [
    for i in range(var.server_count) : "${local.tailscale_prefix}-server-${i}"
  ] : []
}

output "tailscale_agent_hostnames" {
  description = "Tailscale MagicDNS hostnames for agent nodes"
  value = var.enable_tailscale ? [
    for i in range(var.agent_count) : "${local.tailscale_prefix}-agent-${i}"
  ] : []
}

output "kubeconfig_tailscale_command" {
  description = "Command to fetch kubeconfig via Tailscale SSH (when Tailscale is enabled)"
  value       = var.enable_tailscale ? "ssh ubuntu@${local.tailscale_prefix}-server-0 'sudo cat /etc/rancher/k3s/k3s.yaml'" : null
}

output "tailscale_ssh_examples" {
  description = "Example SSH commands using Tailscale hostnames"
  value = var.enable_tailscale ? {
    first_server = "ssh ubuntu@${local.tailscale_prefix}-server-0"
    first_agent  = var.agent_count > 0 ? "ssh ubuntu@${local.tailscale_prefix}-agent-0" : null
  } : null
}

###############################################################################
# Longhorn Backup Outputs
###############################################################################

output "longhorn_backup_enabled" {
  description = "Whether Longhorn backup to Swift S3 is enabled"
  value       = local.longhorn_backup_enabled
}

output "longhorn_backup_container" {
  description = "Swift container name for Longhorn backups"
  value       = local.longhorn_backup_enabled ? openstack_objectstorage_container_v1.longhorn_backup[0].name : null
}

output "longhorn_backup_target" {
  description = "Longhorn backup target URL"
  value       = local.longhorn_backup_enabled ? local.longhorn_backup_target : null
}

output "longhorn_s3_endpoint" {
  description = "S3 endpoint URL for Longhorn backups"
  value       = local.longhorn_backup_enabled ? local.longhorn_s3_endpoint : null
}

