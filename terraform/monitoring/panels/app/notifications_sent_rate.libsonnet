local defaults = import '../../grafonnet-lib/defaults.libsonnet';
local grafana = import '../../grafonnet-lib/grafana.libsonnet';

local panels = grafana.panels;
local targets = grafana.targets;

{
  new(ds, vars)::
    panels.timeseries(
      title='Sent Notifications by Provider',
      datasource=ds.prometheus,
    )
    .configure(
      defaults.configuration.timeseries
      .withUnit('cps')
    )

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum(rate(sent_fcm_notifications_total[$__rate_interval]))',
      legendFormat='FCM (Legacy)',
      exemplar=true,
      refId='SentFcmNotifications',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum(rate(sent_fcm_v1_notifications_total[$__rate_interval]))',
      legendFormat='FCM v1',
      exemplar=true,
      refId='SentFcmV1Notifications',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum(rate(sent_apns_notifications_total[$__rate_interval]))',
      legendFormat='APNS',
      exemplar=true,
      refId='SentApnsNotifications',
    )),
}

