# SSH Keypair Resource
resource "openstack_compute_keypair_v2" "keypair" {
  name       = "${local.resource_prefix}-keypair"
  public_key = file(var.ssh_public_key_path)
}
