################################################################################
#
# Install cargo-chef
#
################################################################################
FROM               rust:bookworm AS chef

WORKDIR             /app
RUN                 cargo install cargo-chef --locked

################################################################################
#
# Generate recipe file
#
################################################################################
FROM                chef AS plan

WORKDIR             /app
COPY                Cargo.lock Cargo.toml ./
COPY                src ./src
COPY                crates ./crates
RUN                 cargo chef prepare --recipe-path recipe.json

################################################################################
#
# Build the binary
#
################################################################################
FROM                chef AS build

ENV                 TINI_VERSION v0.19.0

# This is a build requirement of `opentelemetry-otlp`. Once the new version
# is rolled out, which no longer requires the `protoc`, we'll be able to
# get rid of this.
RUN                 apt-get update \
  && apt-get install -y --no-install-recommends protobuf-compiler

RUN                 apt-get update \
  && apt-get install -y --no-install-recommends lld llvm

WORKDIR             /app
# Cache dependencies
COPY --from=plan    /app/recipe.json recipe.json
COPY --from=plan    /app/crates ./crates

# Install init to be used in runtime container
ADD                 https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini-static /tini
RUN                 chmod +x /tini

RUN                 cargo chef cook --recipe-path recipe.json --release
# Build the local binary
COPY                . .
RUN                 cargo build --bin echo-server --release

################################################################################
#
# Runtime image
#
################################################################################
FROM               debian:bookworm-slim AS runtime

COPY --from=build   /tini /tini

WORKDIR             /app
COPY --from=build   /app/target/release/echo-server /usr/local/bin/echo-server
RUN                 apt-get update \
                        && apt-get install -y --no-install-recommends ca-certificates libssl-dev \
                        && apt-get clean \
                        && rm -rf /var/lib/apt/lists/*

USER                1001:1001
ENTRYPOINT          ["/tini", "--"]
CMD                 ["/usr/local/bin/echo-server"]
