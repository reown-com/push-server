variable "region" {
  type    = string
  default = "eu-central-1"
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
  type = string
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
