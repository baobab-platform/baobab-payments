# Local iteration only: a host-built release binary in the production base.
# CI builds the repository's own Dockerfile instead.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:e792ab3d241a468a4fd7519ddbbebe66b49b5f365771716ea688ad40b6c6f1c2
COPY baobab-payments /usr/local/bin/baobab-payments
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/baobab-payments"]
