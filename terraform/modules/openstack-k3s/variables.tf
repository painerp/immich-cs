# OpenStack K3s Cluster Module - Input Variables
###############################################################################
# Required Variables
###############################################################################
variable "cluster_name" {
  description = "Name of the K3s cluster"
  type        = string
}
variable "k3s_token" {
  description = "K3s cluster token for node authentication"
  type        = string
  sensitive   = true
}
variable "ssh_public_key_path" {
  description = "Path to SSH public key file"
  type        = string
}
###############################################################################
# OpenStack Authentication
###############################################################################
variable "openstack_auth" {
  description = "OpenStack authentication credentials"
  type = object({
    auth_url    = string
    username    = string
    password    = string
    tenant_name = string
    region      = string
    cacert_file = optional(string, null)
  })
  sensitive = true
}
###############################################################################
# Optional Configuration with Defaults
###############################################################################
variable "server_count" {
  description = "Number of control plane servers"
  type        = number
  default     = 3
  validation {
    condition     = var.server_count >= 1 && var.server_count <= 5
    error_message = "Server count must be between 1 and 5"
  }
}
variable "agent_count" {
  description = "Number of worker agents"
  type        = number
  default     = 3
}
variable "server_flavor" {
  description = "OpenStack flavor for server nodes"
  type        = string
  default     = "m1.medium"
}
variable "agent_flavor" {
  description = "OpenStack flavor for agent nodes"
  type        = string
  default     = "m1.medium"
}
variable "bastion_flavor" {
  description = "OpenStack flavor for bastion host"
  type        = string
  default     = "m1.small"
}
variable "image_name" {
  description = "OpenStack image name for instances"
  type        = string
  default     = "ubuntu-24.04-noble-server-cloud-image-amd64"
}
variable "network_cidr" {
  description = "CIDR block for the private network"
  type        = string
  default     = "192.168.255.0/24"
}
variable "dns_servers" {
  description = "DNS server IPs for the subnet"
  type        = list(string)
  default     = ["10.33.16.100", "8.8.8.8"]
}
variable "floating_ip_pool" {
  description = "Name of the floating IP pool"
  type        = string
  default     = "ext_net"
}
variable "enable_bastion" {
  description = "Whether to create a bastion host"
  type        = bool
  default     = true
}
variable "enable_load_balancer" {
  description = "Whether to create a load balancer for HA"
  type        = bool
  default     = true
}
variable "external_ssh_cidrs" {
  description = "CIDR blocks allowed to SSH to servers (0.0.0.0/0 for any)"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}
variable "external_api_cidrs" {
  description = "CIDR blocks allowed to access K8s API (0.0.0.0/0 for any)"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}
variable "tags" {
  description = "Tags to apply to all resources"
  type        = map(string)
  default     = {}
}

###############################################################################
# Tailscale Configuration
###############################################################################

variable "enable_tailscale" {
  description = "Whether to install and configure Tailscale VPN on all nodes"
  type        = bool
  default     = false
}

variable "tailscale_api_key" {
  description = "Tailscale API key (OAuth client secret) for auto-generating ephemeral auth keys per node"
  type        = string
  sensitive   = true
  default     = ""
}

variable "tailscale_tailnet" {
  description = "Tailscale tailnet identifier (organization name or email)"
  type        = string
  default     = ""
}

variable "tailscale_hostname_prefix" {
  description = "Prefix for Tailscale hostnames (will append -server-N or -agent-N). If empty, uses cluster_name"
  type        = string
  default     = ""
}

variable "tailscale_key_expiry" {
  description = "Expiry time for auto-generated auth keys in seconds (default: 7200 = 2 hours)"
  type        = number
  default     = 7200
}

variable "tailscale_ip_update_interval" {
  description = "Interval in seconds for checking Tailscale IP changes (default: 300 = 5 minutes)"
  type        = number
  default     = 300
}

###############################################################################
# Cloud Controller Configuration
###############################################################################

variable "lb_provider" {
  description = "OpenStack load balancer provider (e.g., amphora, octavia)"
  type        = string
  default     = "octavia"
}

variable "insecure" {
  description = "Whether to disable TLS certificate verification for OpenStack API calls"
  type        = bool
  default     = false
}

###############################################################################
# CSI Cinder Storage Configuration
###############################################################################

variable "enable_csi_cinder" {
  description = "Whether to enable OpenStack Cinder CSI driver for persistent volumes"
  type        = bool
  default     = true
}

variable "cinder_default_reclaim_policy" {
  description = "Default reclaim policy for Cinder storage classes (Retain or Delete)"
  type        = string
  default     = "Retain"
  validation {
    condition     = contains(["Retain", "Delete"], var.cinder_default_reclaim_policy)
    error_message = "Reclaim policy must be either 'Retain' or 'Delete'"
  }
}

###############################################################################
# NVIDIA GPU Support Configuration
###############################################################################

variable "enable_nvidia_gpu_operator" {
  description = "Whether to deploy NVIDIA GPU Operator for automatic GPU support"
  type        = bool
  default     = true
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
  description = "ArgoCD admin password (bcrypt hashed or plaintext)"
  type        = string
  sensitive   = true
  default     = ""
}

variable "argocd_repo_url" {
  description = "Git repository URL for ArgoCD applications GitHub URL"
  type        = string
  default     = ""
}

