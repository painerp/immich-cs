# OpenStack K3s Cluster Module - Provider Versions
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
