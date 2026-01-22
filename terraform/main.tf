# K3s Multi-Cloud HA Cluster
# Root orchestration file
terraform {
  required_version = ">= 1.0"
}
# Global locals
locals {
  cluster_name = var.cluster_name
  # Common tags for all resources
  common_tags = {
    managed_by = "terraform"
    cluster    = local.cluster_name
    project    = "k3s-ha-cluster"
  }
}
###############################################################################
# OpenStack K3s Cluster Module
###############################################################################
module "openstack_k3s" {
  source = "./modules/openstack-k3s"
  count  = var.enable_openstack ? 1 : 0
  # Cluster configuration
  cluster_name = "${local.cluster_name}-openstack"
  k3s_token    = var.k3s_token
  # SSH configuration
  ssh_public_key_path = var.ssh_key_path
  # OpenStack authentication
  openstack_auth = {
    auth_url    = var.openstack_auth_url
    username    = var.user_name
    password    = var.user_password
    tenant_name = var.tenant_name
    region      = var.openstack_region
    cacert_file = var.openstack_cacert_file
  }
  # Cluster sizing
  server_count = var.openstack_server_count
  agent_count  = var.openstack_agent_count
  # Instance flavors
  server_flavor  = var.openstack_server_flavor
  agent_flavor   = var.openstack_agent_flavor
  bastion_flavor = var.openstack_bastion_flavor
  # Network configuration
  network_cidr     = var.openstack_network_cidr
  dns_servers      = var.openstack_dns_servers
  floating_ip_pool = var.openstack_floating_ip_pool
  # Security configuration
  external_ssh_cidrs = var.external_ssh_cidrs
  external_api_cidrs = var.external_api_cidrs
  # Feature flags
  enable_bastion       = var.enable_bastion
  enable_load_balancer = var.enable_load_balancer

  # Cloud controller configuration
  insecure    = var.openstack_insecure
  lb_provider = var.openstack_lb_provider

  # Tailscale configuration
  enable_tailscale             = var.enable_tailscale
  tailscale_api_key            = var.tailscale_api_key
  tailscale_tailnet            = var.tailscale_tailnet
  tailscale_hostname_prefix    = var.tailscale_hostname_prefix
  tailscale_key_expiry         = var.tailscale_key_expiry
  tailscale_ip_update_interval = var.tailscale_ip_update_interval
  tailscale_oauth_client_id     = var.tailscale_oauth_client_id
  tailscale_oauth_client_secret = var.tailscale_oauth_client_secret

  # Longhorn distributed storage
  enable_longhorn             = var.enable_longhorn
  longhorn_storage_size       = var.longhorn_storage_size
  longhorn_replica_count      = var.longhorn_replica_count
  enable_longhorn_backup      = var.enable_longhorn_backup
  longhorn_backup_s3_endpoint = var.longhorn_backup_s3_endpoint
  longhorn_backup_s3_region   = var.longhorn_backup_s3_region
  longhorn_backup_schedule    = var.longhorn_backup_schedule
  longhorn_backup_retention   = var.longhorn_backup_retention
  longhorn_backup_concurrency = var.longhorn_backup_concurrency

  # GPU support
  enable_nvidia_gpu_operator = var.enable_nvidia_gpu_operator

  # ArgoCD GitOps
  enable_argocd         = var.enable_argocd
  argocd_admin_password = var.argocd_admin_password
  argocd_repo_url       = var.argocd_repo_url
  argocd_repo_branch    = var.argocd_repo_branch

  # Tags
  tags = merge(local.common_tags, {
    cloud_provider = "openstack"
  })
}