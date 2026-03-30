#!/usr/bin/env sh

set -eu

info()      { printf ' [info] %b\n' "$1"; }
announce()  { printf ' [test] %b\n' "$1"; }
ok()        { printf " [ OK ] %b\n" "$1"; }
fail()      { printf " [FAIL] %b\n" "$1"; exit 1; }

try() {
    actual="$1"
    expected="$2"
    operator="${3:-=}"
    fail_message="${3:-}"
    ok_message="${4:-}"

    # shellcheck disable=1073,1072,1009
    if [ "$actual" "$operator" "$expected" ]; then
        ok "$ok_message"
    else
        fail_message=${fail_message:+": $fail_message"}
        fail "Expected <$expected>, got <$actual>$fail_message"
    fi
}

announce "sudo works"
whoami=$(whoami)
sudo_whoami=$(sudo whoami)
echo try "$whoami" "$sudo_whoami" !=
try "$whoami" "$sudo_whoami" !=
echo try "$sudo_whoami" root
try "$sudo_whoami" root

info "Updating apt packages"
sudo apt-get update >/dev/null

announce "Manually installed packages are the only packages in 'tori manual'"
sudo apt-get install -y sudo >/dev/null 2>&1
tori_manual=$(tori manual | sort)
try "$tori_manual" "sudo"
