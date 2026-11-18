#!/bin/sh
# Print the machine a benchmark ran on, as one line: the CPU's name, then the
# Docker engine's version, operating system, CPU count and memory, as `docker
# info` gives them. `make bench:compare` and `make bench:cli` pass it to the
# report as BENCH_HOST.
#
#   scripts/bench-host.sh
set -u

if [ "$(uname -s)" = Darwin ]; then
    cpu=$(sysctl -n machdep.cpu.brand_string 2>/dev/null)
else
    cpu=$(sed -n 's/^model name[[:space:]]*:[[:space:]]*//p' /proc/cpuinfo 2>/dev/null | head -n 1)
fi
docker=$(docker info --format '{{.ServerVersion}}|{{.OperatingSystem}}|{{.NCPU}}|{{.MemTotal}}' 2>/dev/null)
echo "${cpu:-unknown CPU}|${docker:-unknown}" | awk -F'|' '
    NF == 5 { printf "%s; Docker %s (%s), %s CPUs, %.1f GiB\n", $1, $2, $3, $4, $5 / 1073741824; next }
    { print $1 "; Docker " $2 }'
