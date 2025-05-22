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

# TODO: deprecate this resource
# resource "grafana_dashboard" "at_a_glance_old" {
#   overwrite = true
#   message   = "Updated by Terraform"
#   config_json = jsonencode({
#     "annotations" : {
#       "list" : [
#         {
#           "builtIn" : 1,
#           "datasource" : "-- Grafana --",
#           "enable" : true,
#           "hide" : true,
#           "iconColor" : "rgba(0, 211, 255, 1)",
#           "name" : "Annotations & Alerts",
#           "target" : {
#             "limit" : 100,
#             "matchAny" : false,
#             "tags" : [],
#             "type" : "dashboard"
#           },
#           "type" : "dashboard"
#         }
#       ]
#     },
#     "editable" : true,
#     "fiscalYearStartMonth" : 0,
#     "graphTooltip" : 0,
#     "id" : 37,
#     "links" : [],
#     "liveNow" : false,
#     "panels" : [
#       {
#         "datasource" : {
#           "type" : "prometheus",
#           "uid" : grafana_data_source.prometheus.uid
#         },
#         "description" : "",
#         "fieldConfig" : {
#           "defaults" : {
#             "color" : {
#               "mode" : "thresholds"
#             },
#             "mappings" : [],
#             "thresholds" : {
#               "mode" : "absolute",
#               "steps" : [
#                 {
#                   "color" : "green",
#                   "value" : null
#                 }
#               ]
#             }
#           },
#           "overrides" : []
#         },
#         "gridPos" : {
#           "h" : 8,
#           "w" : 11,
#           "x" : 0,
#           "y" : 0
#         },
#         "id" : 14,
#         "options" : {
#           "colorMode" : "value",
#           "graphMode" : "area",
#           "justifyMode" : "auto",
#           "orientation" : "auto",
#           "reduceOptions" : {
#             "calcs" : [
#               "lastNotNull"
#             ],
#             "fields" : "",
#             "values" : false
#           },
#           "text" : {},
#           "textMode" : "auto"
#         },
#         "pluginVersion" : "8.4.7",
#         "targets" : [
#           {
#             "datasource" : {
#               "type" : "prometheus",
#               "uid" : grafana_data_source.prometheus.uid
#             },
#             "expr" : "sum(rate(received_notifications_total{}[1h]))",
#             "legendFormat" : "__auto",
#             "refId" : "Received"
#           },
#           {
#             "datasource" : {
#               "type" : "prometheus",
#               "uid" : grafana_data_source.prometheus.uid
#             },
#             "editorMode" : "code",
#             "expr" : "sum(increase(sent_apns_notifications_total{}[1h]))",
#             "hide" : false,
#             "legendFormat" : "__auto",
#             "range" : true,
#             "refId" : "SentAPNS"
#           },
#           {
#             "datasource" : {
#               "type" : "prometheus",
#               "uid" : grafana_data_source.prometheus.uid
#             },
#             "editorMode" : "code",
#             "expr" : "sum(increase(sent_fcm_notifications_total{}[1h]))",
#             "hide" : false,
#             "legendFormat" : "__auto",
#             "range" : true,
#             "refId" : "SentFCM"
#           }
#         ],
#         "title" : "Notifications per Hour",
#         "type" : "stat"
#       },
#       {
#         "gridPos" : {
#           "h" : 1,
#           "w" : 24,
#           "x" : 0,
#           "y" : 8
#         },
#         "id" : 8,
#         "title" : "Graphs",
#         "type" : "row"
#       },
#       {
#         "datasource" : {
#           "type" : "prometheus",
#           "uid" : grafana_data_source.prometheus.uid
#         },
#         "fieldConfig" : {
#           "defaults" : {
#             "color" : {
#               "mode" : "palette-classic"
#             },
#             "custom" : {
#               "axisLabel" : "Notifications",
#               "axisPlacement" : "auto",
#               "barAlignment" : 0,
#               "drawStyle" : "line",
#               "fillOpacity" : 0,
#               "gradientMode" : "none",
#               "hideFrom" : {
#                 "legend" : false,
#                 "tooltip" : false,
#                 "viz" : false
#               },
#               "lineInterpolation" : "linear",
#               "lineWidth" : 1,
#               "pointSize" : 5,
#               "scaleDistribution" : {
#                 "type" : "linear"
#               },
#               "showPoints" : "auto",
#               "spanNulls" : false,
#               "stacking" : {
#                 "group" : "A",
#                 "mode" : "none"
#               },
#               "thresholdsStyle" : {
#                 "mode" : "off"
#               }
#             },
#             "mappings" : [],
#             "thresholds" : {
#               "mode" : "absolute",
#               "steps" : [
#                 {
#                   "color" : "green",
#                   "value" : null
#                 }
#               ]
#             },
#             "unit" : "none"
#           },
#           "overrides" : [
#             {
#               "__systemRef" : "hideSeriesFrom",
#               "matcher" : {
#                 "id" : "byNames",
#                 "options" : {
#                   "mode" : "exclude",
#                   "names" : [
#                     "sum(received_notifications{})"
#                   ],
#                   "prefix" : "All except:",
#                   "readOnly" : true
#                 }
#               },
#               "properties" : [
#                 {
#                   "id" : "custom.hideFrom",
#                   "value" : {
#                     "legend" : false,
#                     "tooltip" : false,
#                     "viz" : true
#                   }
#                 }
#               ]
#             }
#           ]
#         },
#         "gridPos" : {
#           "h" : 8,
#           "w" : 11,
#           "x" : 0,
#           "y" : 9
#         },
#         "id" : 10,
#         "options" : {
#           "legend" : {
#             "calcs" : [],
#             "displayMode" : "list",
#             "placement" : "bottom"
#           },
#           "tooltip" : {
#             "mode" : "single",
#             "sort" : "none"
#           }
#         },
#         "targets" : [
#           {
#             "datasource" : {
#               "type" : "prometheus",
#               "uid" : grafana_data_source.prometheus.uid
#             },
#             "exemplar" : true,
#             "expr" : "sum(increase(received_notifications_total{}[1h]))",
#             "format" : "time_series",
#             "interval" : "",
#             "legendFormat" : "",
#             "refId" : "Notifications"
#           }
#         ],
#         "title" : "Received Notifications",
#         "type" : "timeseries"
#       },
#       {
#         "datasource" : {
#           "type" : "prometheus",
#           "uid" : grafana_data_source.prometheus.uid
#         },
#         "fieldConfig" : {
#           "defaults" : {
#             "color" : {
#               "mode" : "palette-classic"
#             },
#             "custom" : {
#               "axisLabel" : "Clients",
#               "axisPlacement" : "auto",
#               "barAlignment" : 0,
#               "drawStyle" : "line",
#               "fillOpacity" : 0,
#               "gradientMode" : "none",
#               "hideFrom" : {
#                 "legend" : false,
#                 "tooltip" : false,
#                 "viz" : false
#               },
#               "lineInterpolation" : "linear",
#               "lineWidth" : 1,
#               "pointSize" : 5,
#               "scaleDistribution" : {
#                 "type" : "linear"
#               },
#               "showPoints" : "auto",
#               "spanNulls" : false,
#               "stacking" : {
#                 "group" : "A",
#                 "mode" : "none"
#               },
#               "thresholdsStyle" : {
#                 "mode" : "off"
#               }
#             },
#             "mappings" : [],
#             "thresholds" : {
#               "mode" : "absolute",
#               "steps" : [
#                 {
#                   "color" : "green",
#                   "value" : null
#                 },
#                 {
#                   "color" : "red",
#                   "value" : 80
#                 }
#               ]
#             }
#           },
#           "overrides" : []
#         },
#         "gridPos" : {
#           "h" : 8,
#           "w" : 10,
#           "x" : 11,
#           "y" : 9
#         },
#         "id" : 12,
#         "options" : {
#           "legend" : {
#             "calcs" : [],
#             "displayMode" : "list",
#             "placement" : "bottom"
#           },
#           "tooltip" : {
#             "mode" : "single",
#             "sort" : "none"
#           }
#         },
#         "targets" : [
#           {
#             "datasource" : {
#               "type" : "prometheus",
#               "uid" : grafana_data_source.prometheus.uid
#             },
#             "exemplar" : true,
#             "expr" : "sum(rate(registered_clients_total{}[$__rate_interval]))",
#             "interval" : "",
#             "legendFormat" : "",
#             "refId" : "Clients"
#           }
#         ],
#         "title" : "Client registration rate",
#         "type" : "timeseries"
#       },
#       {
#         "collapsed" : false,
#         "gridPos" : {
#           "h" : 1,
#           "w" : 24,
#           "x" : 0,
#           "y" : 17
#         },
#         "id" : 6,
#         "panels" : [],
#         "title" : "AWS Load Balancer",
#         "type" : "row"
#       },
#       {
#         "datasource" : {
#           "type" : "cloudwatch",
#           "uid" : grafana_data_source.cloudwatch.uid
#         },
#         "fieldConfig" : {
#           "defaults" : {
#             "color" : {
#               "mode" : "palette-classic"
#             },
#             "custom" : {
#               "axisLabel" : "",
#               "axisPlacement" : "auto",
#               "barAlignment" : 0,
#               "drawStyle" : "line",
#               "fillOpacity" : 0,
#               "gradientMode" : "none",
#               "hideFrom" : {
#                 "legend" : false,
#                 "tooltip" : false,
#                 "viz" : false
#               },
#               "lineInterpolation" : "linear",
#               "lineWidth" : 1,
#               "pointSize" : 5,
#               "scaleDistribution" : {
#                 "type" : "linear"
#               },
#               "showPoints" : "auto",
#               "spanNulls" : false,
#               "stacking" : {
#                 "group" : "A",
#                 "mode" : "none"
#               },
#               "thresholdsStyle" : {
#                 "mode" : "off"
#               }
#             },
#             "mappings" : [],
#             "thresholds" : {
#               "mode" : "absolute",
#               "steps" : [
#                 {
#                   "color" : "green",
#                   "value" : null
#                 },
#                 {
#                   "color" : "red",
#                   "value" : 80
#                 }
#               ]
#             }
#           },
#           "overrides" : []
#         },
#         "gridPos" : {
#           "h" : 9,
#           "w" : 7,
#           "x" : 0,
#           "y" : 18
#         },
#         "id" : 2,
#         "options" : {
#           "legend" : {
#             "calcs" : [],
#             "displayMode" : "list",
#             "placement" : "bottom"
#           },
#           "tooltip" : {
#             "mode" : "single",
#             "sort" : "none"
#           }
#         },
#         "targets" : [
#           {
#             "alias" : "",
#             "datasource" : {
#               "type" : "cloudwatch",
#               "uid" : grafana_data_source.cloudwatch.uid
#             },
#             "dimensions" : {
#               "LoadBalancer" : local.load_balancer
#             },
#             "expression" : "",
#             "id" : "",
#             "matchExact" : true,
#             "metricEditorMode" : 0,
#             "metricName" : "RequestCount",
#             "metricQueryType" : 0,
#             "namespace" : "AWS/ApplicationELB",
#             "period" : "",
#             "queryMode" : "Metrics",
#             "refId" : "A",
#             "region" : "default",
#             "sqlExpression" : "",
#             "statistic" : "Sum"
#           }
#         ],
#         "title" : "Requests",
#         "type" : "timeseries"
#       },
#       {
#         "alert" : {
#           "alertRuleTags" : {},
#           "conditions" : [
#             {
#               "evaluator" : {
#                 "params" : [
#                   15
#                 ],
#                 "type" : "gt"
#               },
#               "operator" : {
#                 "type" : "and"
#               },
#               "query" : {
#                 "params" : [
#                   "A",
#                   "5m",
#                   "now"
#                 ]
#               },
#               "reducer" : {
#                 "params" : [],
#                 "type" : "sum"
#               },
#               "type" : "query"
#             },
#             {
#               "evaluator" : {
#                 "params" : [
#                   15
#                 ],
#                 "type" : "gt"
#               },
#               "operator" : {
#                 "type" : "or"
#               },
#               "query" : {
#                 "params" : [
#                   "B",
#                   "5m",
#                   "now"
#                 ]
#               },
#               "reducer" : {
#                 "params" : [],
#                 "type" : "sum"
#               },
#               "type" : "query"
#             }
#           ],
#           "executionErrorState" : "alerting",
#           "frequency" : "1m",
#           "for" : "",
#           "handler" : 1,
#           "name" : "${var.environment} Echo Server 5XX alert",
#           "noDataState" : "no_data",
#           "message" : "Echo server - Prod - 5XX error",
#           "notifications" : var.notification_channels
#         },
#         "datasource" : {
#           "type" : "cloudwatch",
#           "uid" : grafana_data_source.cloudwatch.uid
#         },
#         "fieldConfig" : {
#           "defaults" : {
#             "color" : {
#               "mode" : "palette-classic"
#             },
#             "custom" : {
#               "axisLabel" : "",
#               "axisPlacement" : "auto",
#               "barAlignment" : 0,
#               "drawStyle" : "line",
#               "fillOpacity" : 0,
#               "gradientMode" : "none",
#               "hideFrom" : {
#                 "legend" : false,
#                 "tooltip" : false,
#                 "viz" : false,
#                 "mode" : "dashed",
#               },
#               "lineInterpolation" : "linear",
#               "lineWidth" : 1,
#               "pointSize" : 5,
#               "scaleDistribution" : {
#                 "type" : "linear"
#               },
#               "showPoints" : "auto",
#               "spanNulls" : false,
#               "stacking" : {
#                 "group" : "A",
#                 "mode" : "none"
#               },
#               "thresholdsStyle" : {
#                 "mode" : "off"
#               }
#             },
#             "mappings" : [],
#             "thresholds" : {
#               "mode" : "absolute",
#               "steps" : [
#                 {
#                   "color" : "green",
#                   "value" : null
#                 },
#                 {
#                   "color" : "red",
#                   "value" : 80
#                 }
#               ]
#             }
#           },
#           "overrides" : []
#         },
#         "gridPos" : {
#           "h" : 9,
#           "w" : 7,
#           "x" : 7,
#           "y" : 18
#         },
#         "id" : 3,
#         "options" : {
#           "legend" : {
#             "calcs" : [],
#             "displayMode" : "list",
#             "placement" : "bottom"
#           },
#           "tooltip" : {
#             "mode" : "single",
#             "sort" : "none"
#           }
#         },
#         "targets" : [
#           {
#             "alias" : "",
#             "datasource" : {
#               "type" : "cloudwatch",
#               "uid" : grafana_data_source.cloudwatch.uid
#             },
#             "dimensions" : {
#               "LoadBalancer" : local.load_balancer
#             },
#             "expression" : "",
#             "id" : "",
#             "matchExact" : true,
#             "metricEditorMode" : 0,
#             "metricName" : "HTTPCode_ELB_5XX_Count",
#             "metricQueryType" : 0,
#             "namespace" : "AWS/ApplicationELB",
#             "period" : "",
#             "queryMode" : "Metrics",
#             "refId" : "A",
#             "region" : "default",
#             "sqlExpression" : "",
#             "statistic" : "Sum"
#           },
#           {
#             "alias" : "",
#             "datasource" : {
#               "type" : "cloudwatch",
#               "uid" : grafana_data_source.cloudwatch.uid
#             },
#             "dimensions" : {
#               "LoadBalancer" : local.load_balancer
#             },
#             "expression" : "",
#             "id" : "",
#             "matchExact" : true,
#             "metricEditorMode" : 0,
#             "metricName" : "HTTPCode_Target_5XX_Count",
#             "metricQueryType" : 0,
#             "namespace" : "AWS/ApplicationELB",
#             "period" : "",
#             "queryMode" : "Metrics",
#             "refId" : "B",
#             "region" : "default",
#             "sqlExpression" : "",
#             "statistic" : "Sum"
#           }
#         ],
#         "thresholds" : [
#           {
#             "colorMode" : "critical",
#             "op" : "gt",
#             "value" : 1,
#             "visible" : true
#           }
#         ],
#         "title" : "5XX",
#         "type" : "timeseries"
#       },
#       {
#         "datasource" : {
#           "type" : "cloudwatch",
#           "uid" : grafana_data_source.cloudwatch.uid
#         },
#         "fieldConfig" : {
#           "defaults" : {
#             "color" : {
#               "mode" : "palette-classic"
#             },
#             "custom" : {
#               "axisLabel" : "",
#               "axisPlacement" : "auto",
#               "barAlignment" : 0,
#               "drawStyle" : "line",
#               "fillOpacity" : 0,
#               "gradientMode" : "none",
#               "hideFrom" : {
#                 "legend" : false,
#                 "tooltip" : false,
#                 "viz" : false
#               },
#               "lineInterpolation" : "linear",
#               "lineWidth" : 1,
#               "pointSize" : 5,
#               "scaleDistribution" : {
#                 "type" : "linear"
#               },
#               "showPoints" : "auto",
#               "spanNulls" : false,
#               "stacking" : {
#                 "group" : "A",
#                 "mode" : "none"
#               },
#               "thresholdsStyle" : {
#                 "mode" : "off"
#               }
#             },
#             "mappings" : [],
#             "thresholds" : {
#               "mode" : "absolute",
#               "steps" : [
#                 {
#                   "color" : "green",
#                   "value" : null
#                 },
#                 {
#                   "color" : "red",
#                   "value" : 80
#                 }
#               ]
#             }
#           },
#           "overrides" : []
#         },
#         "gridPos" : {
#           "h" : 9,
#           "w" : 7,
#           "x" : 14,
#           "y" : 18
#         },
#         "id" : 4,
#         "options" : {
#           "legend" : {
#             "calcs" : [],
#             "displayMode" : "list",
#             "placement" : "bottom"
#           },
#           "tooltip" : {
#             "mode" : "single",
#             "sort" : "none"
#           }
#         },
#         "targets" : [
#           {
#             "alias" : "",
#             "datasource" : {
#               "type" : "cloudwatch",
#               "uid" : grafana_data_source.cloudwatch.uid
#             },
#             "dimensions" : {
#               "LoadBalancer" : local.load_balancer
#             },
#             "expression" : "",
#             "id" : "",
#             "matchExact" : true,
#             "metricEditorMode" : 0,
#             "metricName" : "HTTPCode_ELB_4XX_Count",
#             "metricQueryType" : 0,
#             "namespace" : "AWS/ApplicationELB",
#             "period" : "",
#             "queryMode" : "Metrics",
#             "refId" : "A",
#             "region" : "default",
#             "sqlExpression" : "",
#             "statistic" : "Sum"
#           },
#           {
#             "alias" : "",
#             "datasource" : {
#               "type" : "cloudwatch",
#               "uid" : grafana_data_source.cloudwatch.uid
#             },
#             "dimensions" : {
#               "LoadBalancer" : local.load_balancer
#             },
#             "expression" : "",
#             "id" : "",
#             "matchExact" : true,
#             "metricEditorMode" : 0,
#             "metricName" : "HTTPCode_Target_4XX_Count",
#             "metricQueryType" : 0,
#             "namespace" : "AWS/ApplicationELB",
#             "period" : "",
#             "queryMode" : "Metrics",
#             "refId" : "B",
#             "region" : "default",
#             "sqlExpression" : "",
#             "statistic" : "Sum"
#           }
#         ],
#         "title" : "4XX",
#         "type" : "timeseries"
#       }
#     ],
#     "schemaVersion" : 35,
#     "style" : "dark",
#     "tags" : [],
#     "templating" : {
#       "list" : []
#     },
#     "time" : {
#       "from" : "now-6h",
#       "to" : "now"
#     },
#     "timepicker" : {},
#     "timezone" : "",
#     "title" : "${var.app_name} - ${var.environment} - old",
#     "uid" : "${var.app_name}-${var.environment}-old",
#     "version" : 13,
#     "weekStart" : ""
#   })
# }
