#!/usr/bin/env sh

set -eu
suffix=$(printf '%s' "$1" | sed 's/.*\.//')
binary=tori
name="$binary-$suffix"
tag="$binary:$suffix"
shift

podman run \
    --replace \
    --name "$name" \
    --publish 3008:80 \
    --init \
    "$@" \
    "$tag"
