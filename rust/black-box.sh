# This file tests this tori implementation against the Iganaq Napkin Spec v0.2

set -eu

alias tori=target/debug/tori

echo "Basic smoke test on alias"

tori echo OK

echo "A2. 'log' MUST print only if DEBUG is set and MUST be preceded by ' [log] '"

without_debug=$(tori os 2>&1)
with_debug=$(DEBUG=os tori os 2>&1)
test "$without_debug" != "$with_debug"
echo "$with_debug" | grep -Fq " [log] "
echo "$without_debug" | grep -Fqv " [log] "

echo "A3.2. if su_command is unset, the default must be 'su -c'"

echo 'simulate=true' > "$HOME/.config/tori/tori.conf"
DEBUG=1 tori pkg xterm 2>&1 | grep -Fq 'su -c'

echo "A3.3. if su_command is set, su_command must be the set value"

echo 'simulate=true' > "$HOME/.config/tori/tori.conf"
echo 'su_command=sudo' >> "$HOME/.config/tori/tori.conf"
DEBUG=1 tori pkg xterm 2>&1 | grep -Fq 'sudo'

echo "A3.4. [config] su_command must be validated [as path-resolvable and executable]"

echo 'su_command=sudo' > "$HOME/.config/tori/tori.conf"
! which sudo >/dev/null || tori >/dev/null 2>&1
echo 'su_command=sudo' > "$HOME/.config/tori/tori.conf"
! which sudo >/dev/null || tori >/dev/null 2>&1

echo "B2.1. version | -v | --version -> MUST print the version as in v0.8.0"

output=$(tori version)
test "$output" = "v0.8.0"

output=$(tori -v)
test "$output" = "v0.8.0"

output=$(tori --version)
test "$output" = "v0.8.0"

echo "B2.2. help | -h | --help -> MUST print '<long help>'"

output=$(tori help)
test "$output" = "<long help>"

output=$(tori -h)
test "$output" = "<long help>"

output=$(tori --help)
test "$output" = "<long help>"

echo "B2.3. os -> MUST print the os name"

os_name=$(uname -o)
tori_os=$(tori os)
test -n "$os_name"
test -n "$tori_os"
test "$os_name" = "$tori_os"

echo "B2.3. os -> MUST log the contents of /etc/os-release"

tori_os=$(DEBUG=os tori os 2>&1)
test -n "$tori_os"
echo "$tori_os" | grep -qFf /etc/os-release

echo "B2.4. user -> MUST print the output of the 'whoami' command"

whoami=$(whoami)
tori_user=$(tori user)
test -n "$whoami"
test -n "$tori_user"
test "$whoami" = "$tori_user"

echo "B2.6. echo x y z -> MUST print x y z"

output=$(tori echo x y z)
test "$output" = "x y z"

echo "B2.7. echo -> MUST NOT print any output and exit with status code 0"

tori echo

echo "B2.8. [no input] -> MUST NOT print any output and exit with status code 0"

tori

echo "B2.9. [any other input] -> MUST print 'Unrecognized command: [command]\n<short help>' exit with 1,"

output=$(tori unrecognized_command || true)
test "$output" = "Unrecognized command: unrecognized_command
<short help>"
