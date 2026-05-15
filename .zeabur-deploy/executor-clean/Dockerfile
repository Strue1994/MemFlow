FROM rust:1.86-alpine AS builder

WORKDIR /build

RUN apk add --no-cache musl-dev openssl-dev

COPY . .
RUN cargo build --release --package executor

FROM alpine:3.18

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /build/target/release/executor /app/
COPY --from=builder /build/target/release/memflow-cli /app/

RUN mkdir -p /app/data /app/plugins

ENV RUST_LOG=info
ENV PORT=8080
EXPOSE 8080

CMD ["sh", "-c", "/app/executor --db /app/data/workflows.db serve --addr 0.0.0.0:${PORT:-8080}"]
