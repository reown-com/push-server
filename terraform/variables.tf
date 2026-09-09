variable "region" {
  type    = string
  default = "eu-central-1"
}

variable "environment" {
  type    = string
  default = "staging"
}

variable "azs" {
  type    = list(string)
  default = ["eu-central-1a", "eu-central-1b", "eu-central-1c"]
}

variable "public_url" {
  type    = string
  default = "echo.walletconnect.com"
}

variable "grafana_auth" {
  type      = string
  sensitive = true
}

# AMG -> Grafana Cloud migration dual-run: the aliased grafana.cloud provider (provider.tf).
variable "grafana_cloud_url" {
  description = "Grafana Cloud stack URL (Main Org, prod-eu-west-2)."
  type        = string
  default     = "https://walletconnect.grafana.net"
}

variable "grafana_cloud_token" {
  description = "Grafana Cloud service-account token for the aliased grafana.cloud provider. Distinct from var.grafana_auth (AMG). Set as a SENSITIVE TFC workspace variable."
  type        = string
  sensitive   = true
}

variable "image_version" {
  type    = string
  default = ""
}

variable "geoip_db_key" {
  description = "The key to the GeoIP database"
  type        = string
  default     = "GeoLite2-City.mmdb"
}

variable "jwt_secret" {
  type      = string
  sensitive = true
}

variable "relay_public_key" {
  type      = string
  sensitive = true
}

#-------------------------------------------------------------------------------
# Alerting / Monitoring

variable "notification_channels" {
  description = "The notification channels to send alerts to"
  type        = list(any)
  default     = []
}
