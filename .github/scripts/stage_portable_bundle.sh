#!/usr/bin/env bash
set -euo pipefail

binary_path="${1:?binary path is required}"
output_dir="${2:?output directory is required}"
wrapper_path="${3:-}"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../.." && pwd)"

if [[ "${output_dir}" = /* ]]; then
    bundle_root="${output_dir}"
else
    bundle_root="${repo_root}/${output_dir}"
fi

bundle_parent="$(dirname "${bundle_root}")"
mkdir -p "${bundle_parent}"
bundle_root="$(cd "${bundle_parent}" && pwd)/$(basename "${bundle_root}")"

case "${bundle_root}" in
    "${repo_root}"/*) ;;
    *)
        echo "Output directory must stay inside the repository: ${bundle_root}" >&2
        exit 1
        ;;
esac

if [[ "${binary_path}" = /* ]]; then
    binary_source="${binary_path}"
else
    binary_source="${repo_root}/${binary_path}"
fi

if [[ ! -e "${binary_source}" ]]; then
    echo "Binary not found: ${binary_source}" >&2
    exit 1
fi

rm -rf "${bundle_root}"
mkdir -p "${bundle_root}"

cp -R "${binary_source}" "${bundle_root}/"

if [[ -n "${wrapper_path}" ]]; then
    if [[ "${wrapper_path}" = /* ]]; then
        wrapper_source="${wrapper_path}"
    else
        wrapper_source="${repo_root}/${wrapper_path}"
    fi

    if [[ -e "${wrapper_source}" ]]; then
        cp -R "${wrapper_source}" "${bundle_root}/"
    fi
fi

for file_name in README.md CHANGELOG.md LICENSE; do
    if [[ -e "${repo_root}/${file_name}" ]]; then
        cp -R "${repo_root}/${file_name}" "${bundle_root}/"
    fi
done

for directory_name in licenses source internal; do
    if [[ -e "${repo_root}/${directory_name}" ]]; then
        cp -R "${repo_root}/${directory_name}" "${bundle_root}/"
    fi
done

rm -rf "${bundle_root}/internal/temp"
mkdir -p "${bundle_root}/internal/temp"
