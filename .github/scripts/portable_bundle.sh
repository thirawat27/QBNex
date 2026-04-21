#!/usr/bin/env bash
set -euo pipefail

binary_path="${1:-}"
output_dir="${2:-}"

if [[ -z "${binary_path}" || -z "${output_dir}" ]]; then
  echo "usage: $0 <binary-path> <output-dir>" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
mkdir -p "${output_dir}"

copy_if_exists() {
  local source_path="$1"
  local destination_path="$2"

  if [[ -e "${source_path}" ]]; then
    mkdir -p "$(dirname "${destination_path}")"
    cp -R "${source_path}" "${destination_path}"
  fi
}

copy_if_exists "${repo_root}/${binary_path}" "${output_dir}/$(basename "${binary_path}")"
copy_if_exists "${repo_root}/qb.cmd" "${output_dir}/qb.cmd"
copy_if_exists "${repo_root}/internal" "${output_dir}/internal"
copy_if_exists "${repo_root}/licenses" "${output_dir}/licenses"
copy_if_exists "${repo_root}/README.md" "${output_dir}/README.md"
copy_if_exists "${repo_root}/CHANGELOG.md" "${output_dir}/CHANGELOG.md"
