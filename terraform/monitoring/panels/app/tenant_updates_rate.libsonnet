local defaults = import '../../grafonnet-lib/defaults.libsonnet';
local grafana = import '../../grafonnet-lib/grafana.libsonnet';

local panels = grafana.panels;
local targets = grafana.targets;

{
  new(ds, vars)::
    panels.timeseries(
      title='Tenant Configuration Updates Rate',
      datasource=ds.prometheus,
    )
    .configure(
      defaults.configuration.timeseries
      .withUnit('cps')
    )

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(tenant_apns_updates_total[$__rate_interval]))',
      legendFormat='APNS Updates r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='TenantApnsUpdates',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(tenant_fcm_updates_total[$__rate_interval]))',
      legendFormat='FCM Updates r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='TenantFcmUpdates',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(tenant_fcm_v1_updates_total[$__rate_interval]))',
      legendFormat='FCM v1 Updates r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='TenantFcmV1Updates',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum(rate(tenant_apns_updates_total[$__rate_interval])) + sum(rate(tenant_fcm_updates_total[$__rate_interval])) + sum(rate(tenant_fcm_v1_updates_total[$__rate_interval]))',
      legendFormat='Total Updates',
      exemplar=true,
      refId='TotalTenantUpdates',
    )),
}

