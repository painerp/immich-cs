# Global Variables
###############################################################################
# Global Configuration
###############################################################################
variable "cluster_name" {
  description = "Name of the K3s cluster"
  type        = string
  default     = "k3s-multicloud"
}
variable "k3s_token" {
  description = "Shared K3s cluster token (must be same across all clouds)"
  type        = string
  sensitive   = true
}
variable "ssh_key_path" {
  description = "Path to SSH public key file"
  type        = string
}
###############################################################################
# Cloud Provider Toggles
###############################################################################
variable "enable_openstack" {
  description = "Deploy K3s cluster on OpenStack"
  type        = bool
  default     = true
}
###############################################################################
# OpenStack Configuration
###############################################################################
variable "user_name" {
  description = "OpenStack username"
  type        = string
}
variable "user_password" {
  description = "OpenStack password"
  type        = string
  sensitive   = true
}
variable "tenant_name" {
  description = "OpenStack tenant/project name"
  type        = string
}
variable "openstack_auth_url" {
  description = "OpenStack authentication URL"
  type        = string
  default     = "https://private-cloud.informatik.hs-fulda.de:5000/v3"
}
variable "openstack_region" {
  description = "OpenStack region"
  type        = string
  default     = "RegionOne"
}
variable "openstack_cacert_file" {
  description = "Path to OpenStack CA certificate file"
  type        = string
  default     = "./os-trusted-cas"
}
variable "openstack_server_count" {
  description = "Number of K3s servers in OpenStack"
  type        = number
  default     = 3
}
variable "openstack_agent_count" {
  description = "Number of K3s agents in OpenStack"
  type        = number
  default     = 3
}
variable "openstack_server_flavor" {
  description = "OpenStack flavor for server nodes"
  type        = string
  default     = "m1.medium"
}
variable "openstack_agent_flavor" {
  description = "OpenStack flavor for agent nodes"
  type        = string
  default     = "m1.medium"
}
variable "openstack_bastion_flavor" {
  description = "OpenStack flavor for bastion host"
  type        = string
  default     = "m1.small"
}
variable "openstack_network_cidr" {
  description = "CIDR for OpenStack private network"
  type        = string
  default     = "192.168.255.0/24"
}
variable "openstack_dns_servers" {
  description = "DNS servers for OpenStack subnet"
  type        = list(string)
  default     = ["10.33.16.100", "8.8.8.8"]
}
variable "openstack_floating_ip_pool" {
  description = "OpenStack floating IP pool name"
  type        = string
  default     = "ext_net"
}
###############################################################################
# Common Configuration
###############################################################################
variable "enable_bastion" {
  description = "Create bastion hosts in each cloud"
  type        = bool
  default     = true
}
variable "enable_load_balancer" {
  description = "Create load balancers in each cloud"
  type        = bool
  default     = true
}
variable "external_ssh_cidrs" {
  description = "CIDR blocks allowed to SSH (0.0.0.0/0 = any, TODO: restrict)"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}
variable "external_api_cidrs" {
  description = "CIDR blocks allowed to access K8s API (0.0.0.0/0 = any, TODO: restrict)"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

###############################################################################
# Tailscale Configuration
###############################################################################

variable "enable_tailscale" {
  description = "Enable Tailscale VPN on all cluster nodes for cross-cloud connectivity and bastion-free SSH"
  type        = bool
  default     = false
}

variable "tailscale_api_key" {
  description = <<-EOT
    Tailscale API key for auto-generating ephemeral auth keys.

    Create an OAuth client in the Tailscale Admin Console:
    1. Visit: https://login.tailscale.com/admin/settings/oauth
    2. Click "Generate OAuth Client"
    3. Add description: "Terraform K3s Cluster"
    4. Required scopes:
       - devices:write (manage devices)
       - auth_keys:write (generate ephemeral keys)
    5. Copy the client secret (format: tskey-api-xxxxx-yyyyyyyyyyyy)

    The provider will auto-generate ephemeral keys per node with:
    - Ephemeral: Yes (nodes auto-removed on shutdown)
    - Preauthorized: Yes (auto-approve new nodes)
    - Tags: tag:k3s, tag:openstack, tag:server/agent, tag:{cluster_name}
    - Expiry: Configurable (default: 2 hours)
  EOT
  type        = string
  sensitive   = true
}

variable "tailscale_tailnet" {
  description = <<-EOT
    Tailscale tailnet identifier (your organization name or email).

    Examples:
    - Personal account: user@example.com
    - Organization: example.com

    Find this in Tailscale Admin Console → Settings → General → Tailnet name
  EOT
  type        = string
}

variable "tailscale_hostname_prefix" {
  description = "Prefix for Tailscale MagicDNS hostnames. If empty, uses cluster_name. Nodes will be named {prefix}-server-N or {prefix}-agent-N"
  type        = string
  default     = ""
}

variable "tailscale_key_expiry" {
  description = "Expiry time for auto-generated Tailscale auth keys in seconds (default: 7200 = 2 hours). Increase for slow deployments."
  type        = number
  default     = 7200
  validation {
    condition     = var.tailscale_key_expiry >= 300 && var.tailscale_key_expiry <= 86400
    error_message = "Key expiry must be between 5 minutes (300) and 24 hours (86400)."
  }
}

variable "tailscale_ip_update_interval" {
  description = "Interval in seconds for checking Tailscale IP changes (for ephemeral reconnects). Default: 300 (5 minutes)."
  type        = number
  default     = 300
  validation {
    condition     = var.tailscale_ip_update_interval >= 60 && var.tailscale_ip_update_interval <= 3600
    error_message = "IP update interval must be between 60 seconds and 1 hour (3600)."
  }
}

###############################################################################
# OpenStack Cloud Controller Configuration
###############################################################################

variable "openstack_insecure" {
  description = "Disable TLS certificate verification for OpenStack API calls (set to true for self-signed certs)"
  type        = bool
  default     = true
}

variable "openstack_lb_provider" {
  description = "OpenStack load balancer provider (e.g., amphora, octavia)"
  type        = string
  default     = "amphora"
}

###############################################################################
# GPU Support Configuration
###############################################################################
variable "enable_nvidia_gpu_operator" {
  description = "Whether to deploy NVIDIA GPU Operator for automatic GPU support"
  type        = bool
  default     = false
}

###############################################################################
# ArgoCD GitOps Configuration
###############################################################################
variable "enable_argocd" {
  description = "Whether to deploy ArgoCD for GitOps continuous deployment"
  type        = bool
  default     = true
}

variable "argocd_admin_password" {
  description = <<-EOT
    ArgoCD admin password (bcrypt hashed). Generate with:
    htpasswd -nbBC 10 "" 'yourpassword' | tr -d ':\n' | sed 's/$2y/$2a/'
    Or use the default password 'admin' and change it after first login.
  EOT
  type        = string
  sensitive   = true
  default     = ""
}

variable "argocd_repo_url" {
  description = <<-EOT
    Git repository URL for ArgoCD applications:
    - GitHub: https://github.com/username/repository (private repo requires authentication setup)
  EOT
  type        = string
  default     = ""
}
