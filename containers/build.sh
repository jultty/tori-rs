#!/usr/bin/env sh

set -eu
suffix=$(printf '%s' "$1" | sed 's/.*\.//')
binary=tori
tag="$binary:$suffix"
shift

if podman container exists "$tag"; then
    podman stop --time 3 "$tag"
fi

if ! [ -f "../target/debug/$binary" ]; then
    cd ..
    cargo build
    cd -
fi

cp -v ../target/debug/$binary $binary

podman build \
    --tag "$tag" \
    -f "Containerfile.$suffix" "$@"

if [ -f $binary ]; then
    rm -v $binary
fi
