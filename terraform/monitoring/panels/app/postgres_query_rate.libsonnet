local defaults = import '../../grafonnet-lib/defaults.libsonnet';
local grafana = import '../../grafonnet-lib/grafana.libsonnet';

local panels = grafana.panels;
local targets = grafana.targets;

{
  new(ds, vars)::
    panels.timeseries(
      title='Postgres Query Rate',
      datasource=ds.prometheus,
    )
    .configure(
      defaults.configuration.timeseries
      .withUnit('cps')
    )

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum by (aws_ecs_task_revision, name) (rate(postgres_queries_total[$__rate_interval]))',
      legendFormat='{{name}} r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='PostgresQueryRate',
    ))

    .addTarget(targets.prometheus(
      datasource=ds.prometheus,
      expr='sum(rate(postgres_queries_total[$__rate_interval]))',
      legendFormat='r{{aws_ecs_task_revision}}',
      exemplar=true,
      refId='PostgresQueryRateTotal',
    )),
}
