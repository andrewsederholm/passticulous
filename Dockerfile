# syntax=docker/dockerfile:1

# ---- Build stage ----
FROM rust:1-bookworm AS build
WORKDIR /src

# Build dependencies first so they are cached between source changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY assets assets
COPY static static
COPY src src
RUN touch src/main.rs && cargo build --release --locked

# ---- Runtime stage ----
# Distroless: no shell or package manager, glibc only, runs as uid 65532.
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=build /src/target/release/passticulous /usr/local/bin/passticulous

ENV PORT=8080 \
    BIND_ADDR=0.0.0.0
EXPOSE 8080
USER nonroot:nonroot

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/passticulous", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/passticulous"]
