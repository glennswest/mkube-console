FROM rust:1.85-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs && cargo build --release && rm -rf src
COPY src/ src/
COPY templates/ templates/
COPY static/ static/
COPY build.rs .
RUN touch src/main.rs && cargo build --release

FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/release/mkube-console /usr/local/bin/mkube-console
COPY --from=builder /app/static/ /static/
EXPOSE 9090
ENTRYPOINT ["/usr/local/bin/mkube-console"]
CMD ["/etc/mkube-console/config.yaml"]
