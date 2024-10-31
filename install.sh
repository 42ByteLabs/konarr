#!/bin/bash
# This script is to install and setup Konarr on a new machine

set -e

export REPOSITORY="42bytelabs/konarr"
export VERSION="${KONARR_VERSION:-0.1.0}"
export GITHUB_RAW_URL="https://raw.githubusercontent.com/${REPOSITORY}/refs/heads/${VERSION}"

export CONTAINER_ENGINE=""
export COMPOSE=false


while [[ "$#" -gt 0 ]]; do
  case $1 in
    -v=*|--version=*)
        VERSION="${1#*=}"
    ;;
    -e=*|--engine=*)
        CONTAINER_ENGINE="${1#*=}"
    ;;
    *) echo "Unknown parameter passed: $1"; exit 1 ;;
  esac
  shift
done

echo "Installing Konarr $VERSION"

# Auto detect container engine if not provided
if [ -z "$CONTAINER_ENGINE" ]; then
    echo "Auto detecting container engine"
    # Check if docker is installed
    if [ -x "$(command -v docker)" ]; then
        CONTAINER_ENGINE="docker"
    elif [ -x "$(command -v podman)" ]; then
        CONTAINER_ENGINE="podman"
    else
        echo "Neither docker nor podman is installed. Please install one of them."
        exit 1
    fi
fi
# Check if compose is installed
if [ -x "$(command -v $CONTAINER_ENGINE-compose)" ]; then
    COMPOSE=true
else
    echo "Container compose is not installed. Falling back to single container mode."
fi

echo "Container engine  :: $CONTAINER_ENGINE"
echo "Using Compose     :: $COMPOSE"

# Check if VERSION is `latest`
if [ "$VERSION" == "latest" ]; then
    # Check if git is installed
    if [ ! -x "$(command -v git)" ]; then
        echo "Git is not installed. Please install it."
        exit 1
    fi

    VERSION="main"

    git clone \
        --branch "$VERSION" \
        "https://github.com/$REPOSITORY.git" konarr
    cd ./konarr

    git submodule update --init --recursive
else
    if [ "$COMPOSE" = true ]; then
        echo "Running Konarr with ${CONTAINER_ENGINE}-compose"
        curl -L \
            --output ./docker-compose.yml \
            "${GITHUB_RAW_URL}/docker-compose.yml"

        $CONTAINER_ENGINE-compose up -d

    else
        echo "Starting Konarr with single container"
        $CONTAINER_ENGINE run --rm \
            -p 9000:9000 \
            -v ./data:/data \
            -v ./config:/config \
            "ghcr.io/42bytelabs/konarr:${VERSION}"
    fi
fi
 

