FROM rust:1.71 AS builder

WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/rss_autogen_giscus /usr/local/bin/rss_autogen_giscus

LABEL org.opencontainers.image.source="https://github.com/cam-rod/rss_autogen_giscus"
LABEL org.opencontainers.image.description="Image for generating Giscus discussion posts from RSS feeds"
LABEL org.opencontainers.image.license="Apache-2.0"
CMD ["/usr/local/bin/rss_autogen_giscus"]