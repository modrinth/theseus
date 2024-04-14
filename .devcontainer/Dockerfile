FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

RUN apt update && \
    DEBIAN_FRONTEND=noninteractive apt install -y libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

RUN DEBIAN_FRONTEND=noninteractive apt install -y firefox-esr