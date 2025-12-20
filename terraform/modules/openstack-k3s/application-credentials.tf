# OpenStack Application Credentials

resource "openstack_identity_application_credential_v3" "k3s" {
  name        = "${var.cluster_name}-cloud-controller"
  description = "Application credential for K3s cloud controller manager"
}

