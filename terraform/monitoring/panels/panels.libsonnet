{
  app: {
    postgres_query_rate: (import 'app/postgres_query_rate.libsonnet').new,
    postgres_query_latency: (import 'app/postgres_query_latency.libsonnet').new,
  },
}
