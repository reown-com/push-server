module "this" {
  source  = "app.terraform.io/wallet-connect/label/null"
  version = "0.3.2"

  region = var.region
  name   = var.app_name
}
