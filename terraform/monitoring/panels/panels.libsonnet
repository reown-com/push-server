{
  app: {
    notifications_rate: (import 'app/notifications_rate.libsonnet').new,
    notifications_sent_rate: (import 'app/notifications_sent_rate.libsonnet').new,
    registrations_rate: (import 'app/registrations_rate.libsonnet').new,
    tenant_updates_rate: (import 'app/tenant_updates_rate.libsonnet').new,
    suspensions_rate: (import 'app/suspensions_rate.libsonnet').new,
    postgres_query_rate: (import 'app/postgres_query_rate.libsonnet').new,
    postgres_query_latency: (import 'app/postgres_query_latency.libsonnet').new,
  },
}
