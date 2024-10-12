FROM ghcr.io/42bytelabs/konarr-cli:latest

# Install Syft
RUN curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh | sh -s -- -b /usr/local/bin

RUN ["-m", "agent"]
