FROM docker.io/node:lts as client-builder
WORKDIR /app

ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable

COPY package.json pnpm-lock.yaml ./
RUN --mount=type=cache,id=pnpm,target=/pnpm/store pnpm install --frozen-lockfile

COPY . .
RUN pnpm run build

FROM docker.io/lukemathwalker/cargo-chef:latest AS chef
ARG TARGETARCH=amd64

ENV CARGO_TERM_COLOR=always
RUN apt-get update && apt-get install -y curl ca-certificates clang && rm -rf /var/lib/apt/lists/*
WORKDIR /app
RUN mkdir -p /mold

ADD https://github.com/rui314/mold/releases/download/v2.1.0/mold-2.1.0-x86_64-linux.tar.gz /mold/mold-amd64.tar.gz
ADD https://github.com/rui314/mold/releases/download/v2.1.0/mold-2.1.0-aarch64-linux.tar.gz /mold/mold-arm64.tar.gz

RUN tar -xvf /mold/mold-${TARGETARCH}.tar.gz --strip-components 1 -C /mold \
	&& mv /mold/bin/mold /usr/bin/mold \
	&& chmod +x /usr/bin/mold

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder-common
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .

FROM builder-common AS builder-web
RUN cargo build --release --bin dbost

FROM builder-web AS builder-job
ARG BIN_NAME
ARG PACKAGE
RUN cargo build --release --bin "${BIN_NAME}" --package "${PACKAGE}"

FROM docker.io/debian:buster-slim AS runtime
RUN apt-get update && apt-get install -y curl ca-certificates && rm -rf /var/lib/apt/lists/*

# We do not need the Rust toolchain to run the binary!
FROM runtime as job
ARG BIN_NAME
ARG PACKAGE
COPY --from=builder-job "/app/target/release/${BIN_NAME}" /usr/local/bin
ENTRYPOINT ["/usr/local/bin/${BIN_NAME}"]

FROM runtime as web
COPY --from=builder-web /app/target/release/dbost /usr/local/bin
COPY --from=client-builder /app/public /var/www/public
ENV WEB_PUBLIC_PATH=/var/www/public
ENTRYPOINT ["/usr/local/bin/dbost"]
