# FROM ubuntu:24.04 AS builder
#
# RUN apt-get update && apt-get install -y \
#     build-essential \
#     cmake \
#     git \
#     pkg-config \
#     libtool \
#     autoconf \
#     automake \
#     libevent-dev \
#     libboost-all-dev \
#     libunivalue-dev \
#     libminiupnpc-dev \
#     capnproto libcapnp-dev \
#     && rm -rf /var/lib/apt/lists/*
#
# WORKDIR /src
#
# # Clone sv2-tp source
# RUN git clone https://github.com/stratum-mining/sv2-tp.git .
#
# # Build
# RUN cmake -B build
# RUN cmake --build build -j$(nproc)
#

FROM ubuntu:24.04 AS runtime

WORKDIR /app

# COPY --from=builder /src/build/bin/sv2-tp /app/sv2-tp
COPY sv2-tp /app/sv2-tp

# RUN apt-get update && apt-get install -y \
#     capnproto libcapnp-dev \
#     && rm -rf /var/lib/apt/lists/*

RUN chmod +x /app/sv2-tp

ENTRYPOINT ["/app/sv2-tp"]

CMD ["-signet", \
     "-sv2port=8442", \
     "-debug=sv2", \
     "-loglevel=sv2:trace", \
     "-sv2bind=0.0.0.0", \
     "-datadir=/app/bitcoin", \
     "-ipcconnect=unix:/app/bitcoin/signet/node.sock", \
     "-debug=net"]
