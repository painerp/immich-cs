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

variable "tailscale_oauth_client_id" {
  description = <<-EOT
    Tailscale OAuth client ID for the Kubernetes in-cluster Helm chart deployment.
    
    This is SEPARATE from tailscale_api_key and is used by the Tailscale Helm chart
    running inside the cluster to authenticate with Tailscale.

    Create an OAuth client in the Tailscale Admin Console:
    1. Visit: https://login.tailscale.com/admin/settings/oauth
    2. Click "Generate OAuth Client"
    3. Add description: "Kubernetes in-cluster OAuth"
    4. Required scopes: (depends on Tailscale Helm chart requirements)
    5. Copy the client ID

    This will be stored as a Kubernetes Secret in the tailscale namespace.
  EOT
  type        = string
  sensitive   = true
  default     = ""
}

variable "tailscale_oauth_client_secret" {
  description = <<-EOT
    Tailscale OAuth client secret for the Kubernetes in-cluster Helm chart deployment.
    
    This is SEPARATE from tailscale_api_key and is used by the Tailscale Helm chart
    running inside the cluster to authenticate with Tailscale.

    This will be stored as a Kubernetes Secret in the tailscale namespace.
  EOT
  type        = string
  sensitive   = true
  default     = ""
}

###############################################################################
# Cloudflare Tunnel Configuration
###############################################################################

variable "enable_cloudflare_tunnel" {
  description = "Enable Cloudflare Tunnel for public access to cluster services"
  type        = bool
  default     = false
}

variable "cloudflare_account_id" {
  description = <<-EOT
    Cloudflare account ID for the tunnel.

    Find this in the Cloudflare dashboard URL:
    https://dash.cloudflare.com/<account-id>/ or https://one.dash.cloudflare.com/<account-id>/

    This is not sensitive and can be committed to Git.
    Required if enable_cloudflare_tunnel is true.
  EOT
  type        = string
  default     = ""
}

variable "cloudflare_tunnel_id" {
  description = <<-EOT
    Cloudflare Tunnel ID (UUID).

    Create a tunnel in Cloudflare Zero Trust:
    1. Visit: https://one.dash.cloudflare.com/
    2. Navigate to Networks > Tunnels
    3. Click "Create a tunnel"
    4. Name it (e.g., "immich-tunnel")
    5. Copy the Tunnel ID (UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)

    This is not sensitive and can be committed to Git.
    Required if enable_cloudflare_tunnel is true.
  EOT
  type        = string
  default     = ""
}

variable "cloudflare_tunnel_secret" {
  description = <<-EOT
    Cloudflare Tunnel secret/credentials.

    This is the sensitive tunnel credentials secret from the tunnel you created.
    You can find this:
    - In the tunnel credentials JSON file (download from tunnel settings)
    - Or from the connector installation command shown when creating the tunnel

    This is SENSITIVE and should NOT be committed to Git.
    Store this in terraform.tfvars (which is gitignored).

    Required if enable_cloudflare_tunnel is true.
  EOT
  type        = string
  sensitive   = true
  default     = ""
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
# Longhorn Distributed Storage Configuration
###############################################################################
variable "enable_longhorn" {
  description = "Whether to deploy Longhorn distributed storage system for ReadWriteMany volumes"
  type        = bool
  default     = true
}

variable "longhorn_storage_size" {
  description = "Size of dedicated Cinder volume for Longhorn storage on each agent node (in GiB)"
  type        = number
  default     = 50
}

variable "longhorn_replica_count" {
  description = "Default number of replicas for Longhorn volumes (should match agent count for best HA)"
  type        = number
  default     = 3
}

variable "enable_longhorn_backup" {
  description = "Whether to enable S3-compatible backup target for Longhorn using OpenStack Swift (enabled by default when Longhorn is enabled)"
  type        = bool
  default     = true
}

variable "longhorn_backup_s3_endpoint" {
  description = <<-EOT
    S3 endpoint URL for Longhorn backups (OpenStack Swift S3 API)
  EOT
  type        = string
  default     = ""
}

variable "longhorn_backup_s3_region" {
  description = "S3 region for Longhorn backups (typically matches OpenStack region)"
  type        = string
  default     = "RegionOne"
}

variable "longhorn_backup_schedule" {
  description = "Cron schedule for automatic Longhorn backups (default: daily at 2 AM UTC)"
  type        = string
  default     = "0 2 * * *"
}

variable "longhorn_backup_retention" {
  description = "Number of backup snapshots to retain (default: 7 days)"
  type        = number
  default     = 7
}

variable "longhorn_backup_concurrency" {
  description = "Number of concurrent backups allowed"
  type        = number
  default     = 2
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
  default     = false
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

variable "argocd_repo_branch" {
  description = "Git branch to track for ArgoCD applications"
  type        = string
  default     = "main"
}
