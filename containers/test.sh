#!/usr/bin/env sh

set -eu

info()      { printf ' [info] %b\n' "$1"; }
announce()  { printf ' [test] %b\n' "$1"; }
ok()        { printf " [ OK ] %b\n" "$1"; }
fail()      { printf " [FAIL] %b\n" "$1"; exit 1; }

try() {
    actual="$1"
    expected="$(printf '%b' "$2")"
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

info "tori version $(tori version)"

announce "sudo works"
whoami=$(whoami)
sudo_whoami=$(sudo whoami)
try "$whoami" "$sudo_whoami" !=
try "$sudo_whoami" root

info "Updating apt packages"
sudo apt-get update >/dev/null

announce "Manually installed packages are the only packages in 'tori manual'"
info "Installing: sudo"
sudo apt-get install -y sudo >/dev/null 2>&1
tori_manual=$(tori manual)
try "$tori_manual" "sudo"

announce "Manually installed packages change after installing one"
info "Installing: figlet"
sudo apt-get install -y figlet >/dev/null 2>&1
tori_manual=$(tori manual)
try "$tori_manual" "figlet\nsudo"

announce "Manually installed packages change after installing several"
info "Installing: vim-tiny tmux qalc"
sudo apt-get install -y vim-tiny tmux qalc >/dev/null 2>&1
tori_manual=$(tori manual)
try "$tori_manual" "figlet\nqalc\nsudo\ntmux\nvim-tiny"

announce "Manually installed packages change after uninstalling one"
info "Uninstalling: qalc"
sudo apt-get remove -y qalc >/dev/null 2>&1
tori_manual=$(tori manual)
try "$tori_manual" "figlet\nsudo\ntmux\nvim-tiny"

announce "Manually installed packages change after uninstalling several"
info "Uninstalling: figlet tmux vim-tiny"
sudo apt-get remove -y figlet tmux vim-tiny >/dev/null 2>&1
tori_manual=$(tori manual)
try "$tori_manual" "sudo"
