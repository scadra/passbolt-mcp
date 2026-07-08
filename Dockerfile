# passbolt-mcp over stdio: docker run -i --rm -v ~/.config/passbolt-mcp:/config ghcr.io/scadra/passbolt-mcp
FROM rust:1.88-slim-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home-dir /config mcp
COPY --from=build /src/target/release/passbolt-mcp /usr/local/bin/passbolt-mcp
USER mcp
ENV PASSBOLT_MCP_CONFIG=/config/passbolt-mcp.toml
VOLUME /config
ENTRYPOINT ["passbolt-mcp"]
CMD ["serve"]
