FROM golang:1.24-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -ldflags="-s -w" -o mkube-console ./cmd/mkube-console

FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/mkube-console /usr/local/bin/mkube-console
EXPOSE 9090
ENTRYPOINT ["/usr/local/bin/mkube-console"]
CMD ["-config", "/etc/mkube-console/config.yaml"]
