locals {
  # Turns the arn into the format expected by
  # the Grafana provider e.g.
  # net/prod-relay-load-balancer/e9a51c46020a0f85
  load_balancer = join("/", slice(split("/", var.load_balancer_arn), 1, 4))
}

module "monitoring-role" {
  source          = "app.terraform.io/wallet-connect/monitoring-role/aws"
  version         = "1.1.0"
  context         = module.this
  remote_role_arn = var.monitoring_role_arn
}

resource "grafana_data_source" "prometheus" {
  type = "prometheus"
  name = "${var.app_name}-amp"
  url  = "https://aps-workspaces.eu-central-1.amazonaws.com/workspaces/${var.prometheus_workspace_id}/"

  json_data_encoded = jsonencode({
    httpMethod         = "GET"
    manageAlerts       = false
    sigV4Auth          = true
    sigV4AuthType      = "ec2_iam_role"
    sigV4Region        = "eu-central-1"
    sigV4AssumeRoleArn = module.monitoring-role.iam_role_arn
  })
}

resource "grafana_data_source" "cloudwatch" {
  type = "cloudwatch"
  name = "${var.app_name}-cloudwatch"

  json_data_encoded = jsonencode({
    defaultRegion = "eu-central-1"
    assumeRoleArn = module.monitoring-role.iam_role_arn
  })
}

data "jsonnet_file" "dashboard" {
  source = "${path.module}/dashboard.jsonnet"

  ext_str = {
    dashboard_title = "Push Server - ${title(var.environment)}"
    dashboard_uid   = "push-${var.environment}"

    prometheus_uid = grafana_data_source.prometheus.uid
    cloudwatch_uid = grafana_data_source.cloudwatch.uid

    environment   = var.environment
    notifications = jsonencode(var.notification_channels)
  }
}

resource "grafana_dashboard" "push_server" {
  overwrite   = true
  message     = "Updated by Terraform"
  config_json = data.jsonnet_file.dashboard.rendered
}

################################################################
################################################################
# GRAFANA CLOUD TWINS (migration dual-run)
# Byte-identical copies of the AMG resources above,
# provider = grafana.cloud. AMG blocks above remain the live
# paging path until this unit's route source-flip. At AMG
# teardown the AMG blocks above are deleted; these twins remain.
################################################################
################################################################
# NOTE: push is DASHBOARDS-ONLY (0 alert rules, no folder, no rule group, no incident
# route). There is no later route source-flip for this unit — it completes at dashboard
# parity + sign-off. The banner's "paging path" wording is the shared template; push does
# not page. The AMG blocks above are still deleted at teardown; these twins remain.

# Dedicated CREATE_ROLE for Grafana Cloud. NOT ADD_TRUST on the AMG-era
# eu-central-1-<env>-push-monitoring role: that role is destroyed at AMG teardown, which
# would break Cloud. This Cloud-only role survives teardown. It carries BOTH CloudWatch
# read AND aps:Query* (push has an AMP datasource).
# CONFUSED-DEPUTY GUARD: the trust pins sts:ExternalId so only our Grafana Cloud stack can
# assume it. Principal + ExternalId read from the live mx-prod-grafana-cloudwatch trust.
locals {
  grafana_cloud_role_name = "push-${var.environment}-grafana-cloudwatch"
  cloud_prometheus_uid    = { prod = "wBlpDqaNk", staging = "7EUBUu-Nk" }[var.environment]
  cloud_cloudwatch_uid    = { prod = "MqQtD3aNz", staging = "b6yf8X-Nk" }[var.environment]
}

resource "aws_iam_role" "grafana_cloud" {
  name = local.grafana_cloud_role_name

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect    = "Allow"
        Principal = { AWS = var.grafana_cloud_principal_arn }
        Action    = "sts:AssumeRole"
        Condition = {
          StringEquals = { "sts:ExternalId" = var.grafana_cloud_external_id }
        }
      }
    ]
  })

  tags = module.this.tags
}

resource "aws_iam_role_policy_attachment" "grafana_cloud_cloudwatch" {
  role       = aws_iam_role.grafana_cloud.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonGrafanaCloudWatchAccess"
}

resource "aws_iam_role_policy" "grafana_cloud_amp_read" {
  name = "amp-query"
  role = aws_iam_role.grafana_cloud.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "aps:QueryMetrics",
          "aps:GetLabels",
          "aps:GetSeries",
          "aps:GetMetricMetadata"
        ]
        Resource = var.prometheus_workspace_arn
      }
    ]
  })
}

# Cloud AMP reader — twin of grafana_data_source.prometheus. uid preserved (== AMG uid) so
# the dashboard's prometheus_uid resolves unchanged. Grafana Cloud assumes the dedicated
# role via grafana_assume_role (instead of the AMG monitoring-role's ec2_iam_role).
resource "grafana_data_source" "cloud_prometheus" {
  provider = grafana.cloud

  # Amazon Managed Prometheus datasource plugin, NOT the plain "prometheus" type. On
  # Grafana Cloud only this type performs the AWS SigV4 grafana_assume_role into our
  # account to query AMP; a plain "prometheus" datasource returns 403 (no assume-role),
  # so the dashboard's AMP panels show no data. Matches every working AMP reader on Cloud
  # (prod-pay-core-amp, staging-blockchain-api-amp, central cloudflare/supabase, mx-*).
  type = "grafana-amazonprometheus-datasource"
  name = "${var.app_name}-amp"
  url  = "https://aps-workspaces.eu-central-1.amazonaws.com/workspaces/${var.prometheus_workspace_id}/"
  uid  = local.cloud_prometheus_uid

  json_data_encoded = jsonencode({
    httpMethod         = "GET"
    manageAlerts       = false
    authType           = "grafana_assume_role"
    assumeRoleArn      = aws_iam_role.grafana_cloud.arn
    defaultRegion      = "eu-central-1"
    sigV4Auth          = true
    sigV4AuthType      = "grafana_assume_role"
    sigV4AssumeRoleArn = aws_iam_role.grafana_cloud.arn
    sigV4Region        = "eu-central-1"
  })

  depends_on = [aws_iam_role_policy.grafana_cloud_amp_read]
}

# Cloud CloudWatch — twin of grafana_data_source.cloudwatch. uid preserved.
resource "grafana_data_source" "cloud_cloudwatch" {
  provider = grafana.cloud

  type = "cloudwatch"
  name = "${var.app_name}-cloudwatch"
  uid  = local.cloud_cloudwatch_uid

  json_data_encoded = jsonencode({
    defaultRegion = "eu-central-1"
    authType      = "grafana_assume_role"
    assumeRoleArn = aws_iam_role.grafana_cloud.arn
  })

  depends_on = [aws_iam_role_policy_attachment.grafana_cloud_cloudwatch]
}

# Dashboard twin — same dashboard.jsonnet, uids point at the Cloud datasources
# (teardown-safe). Preserved uids -> byte-identical rendered config_json. No folder.
data "jsonnet_file" "dashboard_cloud" {
  source = "${path.module}/dashboard.jsonnet"

  ext_str = {
    dashboard_title = "Push Server - ${title(var.environment)}"
    dashboard_uid   = "push-${var.environment}"

    prometheus_uid = grafana_data_source.cloud_prometheus.uid
    cloudwatch_uid = grafana_data_source.cloud_cloudwatch.uid

    environment   = var.environment
    notifications = jsonencode(var.notification_channels)
  }
}

resource "grafana_dashboard" "push_server_cloud" {
  provider = grafana.cloud

  overwrite   = true
  message     = "Updated by Terraform"
  config_json = data.jsonnet_file.dashboard_cloud.rendered
}

