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
    libssl-dev

# Set version and base URL for Bitcoin SV2 release
ENV VERSION=sv2-tp-0.1.19
ENV BASE_URL=https://github.com/Sjors/bitcoin/releases/download/${VERSION}

# Download and extract Bitcoin SV2 release
RUN curl -L ${BASE_URL}/bitcoin-${VERSION}-x86_64-linux-gnu.tar.gz -o /tmp/bitcoin-sv2.tar.gz \
    && mkdir /tmp/bitcoin-sv2-extracted \
    && tar -xzf /tmp/bitcoin-sv2.tar.gz -C /tmp/bitcoin-sv2-extracted --strip-components=1 \
    && ls -l /tmp/bitcoin-sv2-extracted/bin/  # Debug: Check if binaries are in the extracted folder

# Copy the extracted binaries to /usr/local/bin
RUN cp /tmp/bitcoin-sv2-extracted/bin/* /usr/local/bin/ \
    && chmod +x /usr/local/bin/bitcoind /usr/local/bin/bitcoin-cli /usr/local/bin/bitcoin-tx /usr/local/bin/bitcoin-wallet /usr/local/bin/bitcoin-qt \
    && rm -rf /tmp/bitcoin-sv2.tar.gz /tmp/bitcoin-sv2-extracted

# Volume for Bitcoin data folder
VOLUME ["/root/.bitcoin"]
WORKDIR /root/.bitcoin

# Expose the SV2 port
EXPOSE 8442

# Entry point for Bitcoin SV2
ENTRYPOINT ["bitcoind", "-testnet4", "-sv2", "-sv2port=8442", "-debug=sv2", "-sv2bind=0.0.0.0"]

