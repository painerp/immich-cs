# Aggregated Outputs from All Clouds
###############################################################################
# OpenStack Outputs
###############################################################################
output "openstack_cluster" {
  description = "OpenStack K3s cluster information"
  value = var.enable_openstack ? {
    cluster_name       = module.openstack_k3s[0].cluster_name
    bastion_ip         = module.openstack_k3s[0].bastion_ip
    loadbalancer_ip    = module.openstack_k3s[0].loadbalancer_ip
    server_ips         = module.openstack_k3s[0].server_ips
    agent_ips          = module.openstack_k3s[0].agent_ips
    network_id         = module.openstack_k3s[0].network_id
    kubeconfig_command = module.openstack_k3s[0].kubeconfig_command
  } : null
}
###############################################################################
# Flag Output
###############################################################################
output "openstack_enabled" {
  description = "Whether OpenStack cluster is enabled"
  value       = var.enable_openstack
}
output "enable_nvidia_gpu_operator" {
  description = "Whether NVIDIA GPU Operator is enabled the cluster"
  value       = var.enable_nvidia_gpu_operator
}
output "enable_argocd" {
  description = "Whether ArgoCD is enabled in the cluster"
  value       = var.enable_argocd
}
###############################################################################
# Aggregated Outputs
###############################################################################
output "all_bastion_ips" {
  description = "All bastion IPs across clouds"
  value = compact([
    var.enable_openstack ? module.openstack_k3s[0].bastion_ip : null
  ])
}
output "all_server_ips" {
  description = "All K3s server IPs across clouds"
  value = concat(
    var.enable_openstack ? module.openstack_k3s[0].server_ips : []
  )
}
output "all_agent_ips" {
  description = "All K3s agent IPs across clouds"
  value = concat(
    var.enable_openstack ? module.openstack_k3s[0].agent_ips : []
  )
}
output "primary_api_endpoint" {
  description = "Primary K8s API endpoint"
  value       = var.enable_openstack ? "https://${module.openstack_k3s[0].loadbalancer_ip}:6443" : null
}

###############################################################################
# Tailscale Outputs
###############################################################################

output "tailscale_enabled" {
  description = "Whether Tailscale VPN is enabled across clusters"
  value       = var.enable_tailscale
}

output "tailscale_hostnames" {
  description = "Tailscale MagicDNS hostnames for all nodes across clouds"
  value = var.enable_tailscale ? {
    openstack_servers = var.enable_openstack ? module.openstack_k3s[0].tailscale_server_hostnames : []
    openstack_agents  = var.enable_openstack ? module.openstack_k3s[0].tailscale_agent_hostnames : []
  } : null
}

output "kubeconfig_via_tailscale" {
  description = "Commands to fetch kubeconfig via Tailscale SSH (when enabled)"
  value = var.enable_tailscale ? {
    openstack = var.enable_openstack ? module.openstack_k3s[0].kubeconfig_tailscale_command : null
  } : null
}

###############################################################################
# Longhorn Backup Outputs
###############################################################################

output "longhorn_backup_info" {
  description = "Longhorn backup configuration information"
  sensitive   = true
  value = var.enable_openstack && var.enable_longhorn && var.enable_longhorn_backup ? {
    enabled        = module.openstack_k3s[0].longhorn_backup_enabled
    container_name = module.openstack_k3s[0].longhorn_backup_container
    backup_target  = module.openstack_k3s[0].longhorn_backup_target
    s3_endpoint    = module.openstack_k3s[0].longhorn_s3_endpoint
  } : null
}
