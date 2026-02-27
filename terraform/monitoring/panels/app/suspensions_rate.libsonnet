local defaults = import '../../grafonnet-lib/defaults.libsonnet';
local grafana = import '../../grafonnet-lib/grafana.libsonnet';

local panels = grafana.panels;
local targets = grafana.targets;

{
  new(ds, vars)::
    panels.timeseries(
      title='Suspensions Rate',
      datasource=ds.prometheus,
    )
    .configure(
      defaults.configuration.timeseries
      .withUnit('cps')
    )

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(tenant_suspensions_total[$__rate_interval]))',
      legendFormat='Tenant Suspensions r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='TenantSuspensions',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(client_suspensions_total[$__rate_interval]))',
      legendFormat='Client Suspensions r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='ClientSuspensions',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum(rate(tenant_suspensions_total[$__rate_interval]))',
      legendFormat='Total Tenant Suspensions',
      exemplar=true,
      refId='TotalTenantSuspensions',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum(rate(client_suspensions_total[$__rate_interval]))',
      legendFormat='Total Client Suspensions',
      exemplar=true,
      refId='TotalClientSuspensions',
    )),
}

