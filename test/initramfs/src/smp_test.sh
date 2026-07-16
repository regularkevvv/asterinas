#!/bin/sh

# SPDX-License-Identifier: MPL-2.0

set -eu

expected_cpus="${1:?expected CPU count is required}"
online_cpus="$(nproc)"
cpuinfo_cpus="$(grep -c '^processor[[:space:]]*:' /proc/cpuinfo)"

if [ "${online_cpus}" -ne "${expected_cpus}" ]; then
    echo "Expected ${expected_cpus} online processors, found ${online_cpus}" >&2
    exit 1
fi

if [ "${cpuinfo_cpus}" -ne "${expected_cpus}" ]; then
    echo "Expected ${expected_cpus} /proc/cpuinfo entries, found ${cpuinfo_cpus}" >&2
    exit 1
fi

pids=""
results_dir="$(mktemp -d)"
trap 'rm -rf "${results_dir}"' EXIT
worker_count=$((expected_cpus * 4))
worker=0
while [ "${worker}" -lt "${worker_count}" ]; do
    sh -c '
        result_file="$1"
        value=0
        iteration=0
        while [ "${iteration}" -lt 20000 ]; do
            value=$((value + iteration))
            iteration=$((iteration + 1))
        done
        test "${value}" -eq 199990000
        awk "{ print \$39 }" /proc/self/stat > "${result_file}"
    ' sh "${results_dir}/${worker}" &
    pids="${pids} $!"
    worker=$((worker + 1))
done

failed=0
for pid in ${pids}; do
    if ! wait "${pid}"; then
        failed=1
    fi
done
test "${failed}" -eq 0

cpu=0
while [ "${cpu}" -lt "${expected_cpus}" ]; do
    if ! grep -qx "${cpu}" "${results_dir}"/*; then
        echo "No userspace worker executed on CPU ${cpu}" >&2
        exit 1
    fi
    cpu=$((cpu + 1))
done

echo "SMP test passed with ${expected_cpus} processors."
