FROM debian:bullseye-slim

# Install required packages
RUN apt-get update && apt-get install -y \
    bash \
    ca-certificates \
    curl \
    tar \
    libgcc-s1 \
    libpthread-stubs0-dev \
    libc6 \
    libssl-dev \
 && rm -rf /var/lib/apt/lists/*

# Arguments from buildx for architecture/OS
ARG TARGETARCH
ARG TARGETOS

# Set version and base URL for Bitcoin SV2 release
ENV VERSION=sv2-tp-0.1.19
ENV BASE_URL=https://github.com/Sjors/bitcoin/releases/download/${VERSION}

# Choose the right asset based on architecture
RUN set -eux; \
    case "${TARGETARCH}" in \
        amd64)  ARCH_SUFFIX="x86_64-linux-gnu" ;; \
        arm64)  ARCH_SUFFIX="aarch64-linux-gnu" ;; \
        arm)    ARCH_SUFFIX="arm-linux-gnueabihf" ;; \
        *) echo "Unsupported architecture: ${TARGETARCH}" && exit 1 ;; \
    esac; \
    FILENAME="bitcoin-${VERSION}-${ARCH_SUFFIX}.tar.gz"; \
    echo "Downloading ${FILENAME} for ${TARGETOS}/${TARGETARCH}..."; \
    curl -L "${BASE_URL}/${FILENAME}" -o /tmp/bitcoin-sv2.tar.gz; \
    mkdir /tmp/bitcoin-sv2-extracted; \
    tar -xzf /tmp/bitcoin-sv2.tar.gz -C /tmp/bitcoin-sv2-extracted --strip-components=1; \
    cp /tmp/bitcoin-sv2-extracted/bin/* /usr/local/bin/; \
    chmod +x /usr/local/bin/*; \
    rm -rf /tmp/bitcoin-sv2.tar.gz /tmp/bitcoin-sv2-extracted

# Volume for Bitcoin data folder
VOLUME ["/root/.bitcoin"]
WORKDIR /root/.bitcoin

# Expose the SV2 port
EXPOSE 8442

# Entry point for Bitcoin SV2
ENTRYPOINT ["bitcoind", "-testnet4", "-sv2", "-sv2port=8442", "-debug=sv2", "-sv2bind=0.0.0.0"]

