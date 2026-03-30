#!/usr/bin/env sh

set -eu

info()      { printf ' [info] %b\n' "$1"; }
announce()  { printf ' [test] %b\n' "$1"; }
ok()        { printf " [ OK ] %b\n" "$1"; }
fail()      { printf " [FAIL] %b\n" "$1"; exit 1; }

try() {
    actual="$1"
    expected="$2"
    fail_message="${3:-}"
    ok_message="${4:-}"

    if [ "$actual" = "$expected" ]; then
        ok "$ok_message"
    else
        fail "Expected <$expected>, got <$actual> $fail_message"
    fi
}

announce "Fresh install has no manually installed packages"
tori_manual=$(tori manual)
try "$tori_manual" ""

info "Updating apt packages"
apt-get update >/dev/null

announce "Manually installed package is the only package in 'tori manual'"
apt-get install -y figlet >/dev/null 2>&1
tori_manual=$(tori manual)
try "$tori_manual" figlet

announce "Manually installed packages are the only packages in 'tori manual'"
apt-get install -y sudo >/dev/null 2>&1
tori_manual=$(tori manual | sort)
try "$tori_manual" "$(printf 'figlet\nsudo')"
