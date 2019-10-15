FROM debian:stretch AS toolchain

# To use http/https proxy while building, use:
# docker build --build-arg https_proxy=http://fwdproxy:8080 --build-arg http_proxy=http://fwdproxy:8080

RUN echo "deb http://deb.debian.org/debian stretch-backports main" > /etc/apt/sources.list.d/backports.list \
    && apt-get update && apt-get install -y protobuf-compiler/stretch-backports cmake curl clang \
    && apt-get clean && rm -r /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH "$PATH:/root/.cargo/bin"

WORKDIR /libra
COPY rust-toolchain /libra/rust-toolchain
RUN rustup install $(cat rust-toolchain)

FROM toolchain AS builder

COPY . /libra
RUN cargo build --release -p libra-node -p client -p benchmark && cd target/release && rm -r build deps incremental
RUN strip target/release/client

### Production Image ###
FROM debian:stretch AS prod

RUN mkdir -p /opt/libra/bin /opt/libra/etc
COPY --from=builder /libra/target/release/client /opt/libra/bin/libra_client
COPY scripts/cli/consensus_peers.config.toml /opt/libra/etc/consensus_peers.config.toml

ENTRYPOINT ["/opt/libra/bin/libra_client"]
CMD ["--host", "ac.testnet.libra.org", "--port", "8000", "-s", "/opt/libra/etc/consensus_peers.config.toml"]

ARG BUILD_DATE
ARG GIT_REV
ARG GIT_UPSTREAM

LABEL org.label-schema.schema-version="1.0"
LABEL org.label-schema.build-date=$BUILD_DATE
LABEL org.label-schema.vcs-ref=$GIT_REV
LABEL vcs-upstream=$GIT_UPSTREAM
