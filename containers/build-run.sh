#!/usr/bin/env sh

set -eu

./build.sh "$1" && clear
./run.sh "$1"
