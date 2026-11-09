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
# Product-name alias created by scripts/setup_*.: ./qbnex (symlink or copy of ./qb).
copy_if_exists "${repo_root}/qbnex" "${output_dir}/qbnex"
copy_if_exists "${repo_root}/qbnex.cmd" "${output_dir}/qbnex.cmd"
copy_if_exists "${repo_root}/internal" "${output_dir}/internal"
# Prebuilt Rust runtime (libqb_runtime.a, linked into every program) and QIR runner.
copy_if_exists "${repo_root}/crates/qb-runtime/os" "${output_dir}/crates/qb-runtime/os"
copy_if_exists "${repo_root}/crates/qb-exec/bin" "${output_dir}/crates/qb-exec/bin"
copy_if_exists "${repo_root}/licenses" "${output_dir}/licenses"
copy_if_exists "${repo_root}/README.md" "${output_dir}/README.md"
copy_if_exists "${repo_root}/CHANGELOG.md" "${output_dir}/CHANGELOG.md"
