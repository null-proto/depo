FROM rust:alpine AS builder

COPY . /app
WORKDIR /app
RUN cargo fetch
RUN cargo test
RUN cargo build --release

FROM alpine:edge AS runner
COPY --from=builder /app/target/release/depo /depo
ENTRYPOINT [ "/depo" ]
