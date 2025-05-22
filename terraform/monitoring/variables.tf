variable "region" {
  type    = string
  default = "eu-central-1"
}

variable "app_name" {
  type = string
}

variable "environment" {
  type = string
}

variable "prometheus_workspace_id" {
  type = string
}

variable "load_balancer_arn" {
  type = string
}

variable "notification_channels" {
  description = "The notification channels to send alerts to"
  type        = list(any)
}

variable "monitoring_role_arn" {
  description = "The ARN of the monitoring role."
  type        = string
}
