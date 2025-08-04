# Use Alpine as base image
FROM alpine:3.20

# Install required tools
RUN apk add --no-cache curl jq tar

# Set build arguments
ARG SV2_APP
ARG CONFIG_FILE
ARG GITHUB_REPO

ENV GITHUB_REPO=${GITHUB_REPO}
ENV SV2_APP=${SV2_APP}

# Create app directory
WORKDIR /app

# Download and extract the asset
RUN set -eux; \
    RELEASE_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"; \
    echo "Fetching release info from $RELEASE_URL"; \
    ASSET_URL=$(curl -s $RELEASE_URL | \
        jq -r --arg APP "${SV2_APP}.tar.gz" '.assets[] | select(.name == $APP) | .browser_download_url'); \
    echo "Downloading asset from $ASSET_URL"; \
    curl -L "$ASSET_URL" -o "${SV2_APP}.tar.gz"; \
    tar -xzf "${SV2_APP}.tar.gz" --strip-components=0; \
    rm "${SV2_APP}.tar.gz"; \
    chmod +x "/app/${SV2_APP}"

# Entry point logic
ENTRYPOINT ["/bin/sh", "-c"]

CMD [ "\
cd /app && \
CONFIG_FILE_PATH=\"${CONFIG_FILE}\"; \
if [ -z \"$CONFIG_FILE_PATH\" ]; then \
  echo \"No CONFIG_FILE provided. Looking for fallback...\"; \
  CONFIG_FILE_PATH=$(find . -type f -path \"*/config-examples/*-hosted-*.toml\" | head -n1); \
fi; \
if [ ! -f \"$CONFIG_FILE_PATH\" ]; then \
  echo \"Error: Config file \\\"$CONFIG_FILE_PATH\\\" not found.\"; \
  exit 1; \
fi; \
echo \"Using config file: $CONFIG_FILE_PATH\"; \
exec /app/${SV2_APP}/${SV2_APP} --config \"$CONFIG_FILE_PATH\" \
"]

