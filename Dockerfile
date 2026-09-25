# syntax=docker/dockerfile:1
# baobab-payments: the Baobab Payment API (ADR-PAY-0001) with the sandbox provider.
# Base images are pinned by tag and digest; never latest.

FROM rust:1.98-trixie@sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546 AS build
WORKDIR /src
# Dependencies first, so source changes reuse the cached dependency layer.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && touch src/lib.rs \
 && cargo build --release --locked \
 && rm -rf src
COPY contracts ./contracts
COPY src ./src
RUN touch src/main.rs src/lib.rs \
 && cargo build --release --locked \
 && cp target/release/baobab-payments /baobab-payments

FROM gcr.io/distroless/cc-debian13:nonroot@sha256:54df941ed0d06a1bd95ef5e0ce391fd8d9f94b64782dc9a60062727849ee3f97
ARG VERSION=0.0.0-dev
ARG REVISION=unknown
LABEL org.opencontainers.image.title="baobab-payments" \
      org.opencontainers.image.description="Baobab Payment API (sandbox provider only; HyperSwitch not yet integrated)" \
      org.opencontainers.image.source="https://github.com/baobab-platform/baobab-payments" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.vendor="Baobab Platform"
COPY --from=build /baobab-payments /usr/local/bin/baobab-payments
USER 65532:65532
EXPOSE 8080
ENV HTTP_PORT=8080
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/usr/local/bin/baobab-payments", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/baobab-payments"]
