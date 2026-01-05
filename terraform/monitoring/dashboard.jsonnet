local grafana = import 'grafonnet-lib/grafana.libsonnet';
local panels = import 'panels/panels.libsonnet';

local dashboard = grafana.dashboard;
local row = grafana.row;
local annotation = grafana.annotation;
local layout = grafana.layout;

local ds = {
  prometheus: {
    type: 'prometheus',
    uid: std.extVar('prometheus_uid'),
  },
  cloudwatch: {
    type: 'cloudwatch',
    uid: std.extVar('cloudwatch_uid'),
  },
};
local vars = {
  namespace: 'Push',
  environment: std.extVar('environment'),
  notifications: std.parseJson(std.extVar('notifications')),
};

////////////////////////////////////////////////////////////////////////////////

local height = 8;
local pos = grafana.layout.pos(height);

////////////////////////////////////////////////////////////////////////////////

dashboard.new(
  title=std.extVar('dashboard_title'),
  uid=std.extVar('dashboard_uid'),
  editable=true,
  graphTooltip=dashboard.graphTooltips.sharedCrosshair,
  timezone=dashboard.timezones.utc,
)
.addAnnotation(
  annotation.new(
    target={
      limit: 100,
      matchAny: false,
      tags: [],
      type: 'dashboard',
    },
  )
)
.addPanels(layout.generate_grid([
  //////////////////////////////////////////////////////////////////////////////
  row.new('Notifications'),
  panels.app.notifications_rate(ds, vars) { gridPos: pos._6 },
  panels.app.notifications_sent_rate(ds, vars) { gridPos: pos._6 },

  //////////////////////////////////////////////////////////////////////////////
  row.new('Registrations & Tenants'),
  panels.app.registrations_rate(ds, vars) { gridPos: pos._6 },
  panels.app.tenant_updates_rate(ds, vars) { gridPos: pos._6 },

  //////////////////////////////////////////////////////////////////////////////
  row.new('Suspensions'),
  panels.app.suspensions_rate(ds, vars) { gridPos: pos._12 },

  //////////////////////////////////////////////////////////////////////////////
  row.new('Database'),
  panels.app.postgres_query_rate(ds, vars) { gridPos: pos._6 },
  panels.app.postgres_query_latency(ds, vars) { gridPos: pos._6 },
]))
