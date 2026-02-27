local defaults = import '../../grafonnet-lib/defaults.libsonnet';
local grafana = import '../../grafonnet-lib/grafana.libsonnet';

local panels = grafana.panels;
local targets = grafana.targets;

{
  new(ds, vars)::
    panels.timeseries(
      title='Notifications Rate',
      datasource=ds.prometheus,
    )
    .configure(
      defaults.configuration.timeseries
      .withUnit('cps')
    )

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(received_notifications_total[$__rate_interval]))',
      legendFormat='Received r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='ReceivedNotifications',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(sent_fcm_notifications_total[$__rate_interval]))',
      legendFormat='FCM r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='SentFcmNotifications',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(sent_fcm_v1_notifications_total[$__rate_interval]))',
      legendFormat='FCM v1 r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='SentFcmV1Notifications',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(sent_apns_notifications_total[$__rate_interval]))',
      legendFormat='APNS r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='SentApnsNotifications',
    )),
}

