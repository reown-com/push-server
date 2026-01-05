local defaults = import '../../grafonnet-lib/defaults.libsonnet';
local grafana = import '../../grafonnet-lib/grafana.libsonnet';

local panels = grafana.panels;
local targets = grafana.targets;

{
  new(ds, vars)::
    panels.timeseries(
      title='Registrations Rate',
      datasource=ds.prometheus,
    )
    .configure(
      defaults.configuration.timeseries
      .withUnit('cps')
    )

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(registered_clients_total[$__rate_interval]))',
      legendFormat='Clients r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='RegisteredClients',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision) (rate(registered_tenants_total[$__rate_interval]))',
      legendFormat='Tenants r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='RegisteredTenants',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum(rate(registered_clients_total[$__rate_interval]))',
      legendFormat='Total Clients',
      exemplar=true,
      refId='RegisteredClientsTotal',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum(rate(registered_tenants_total[$__rate_interval]))',
      legendFormat='Total Tenants',
      exemplar=true,
      refId='RegisteredTenantsTotal',
    )),
}

