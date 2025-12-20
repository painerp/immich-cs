# Provider Configuration
terraform {
  required_version = ">= 1.0"
  required_providers {
    openstack = {
      source  = "terraform-provider-openstack/openstack"
      version = ">= 3.0.0"
    }
    tailscale = {
      source  = "tailscale/tailscale"
      version = "~> 0.17"
    }
  }
}

# OpenStack Provider
provider "openstack" {
  user_name   = var.user_name
  tenant_name = var.tenant_name
  password    = var.user_password
  auth_url    = var.openstack_auth_url
  region      = var.openstack_region
  cacert_file = var.openstack_cacert_file
}

# Tailscale Provider
# Used to auto-generate ephemeral auth keys per node
provider "tailscale" {
  api_key = var.tailscale_api_key
  tailnet = var.tailscale_tailnet
}
