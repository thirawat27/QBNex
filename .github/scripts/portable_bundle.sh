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
# Files the compiler reads at run time next to itself: the $IMPORT standard library and
# the support includes ($COLOR, vwatch, the SUB _GL table compiler/support/opengl/gl_table.txt).
copy_if_exists "${repo_root}/compiler/stdlib" "${output_dir}/compiler/stdlib"
copy_if_exists "${repo_root}/compiler/support" "${output_dir}/compiler/support"
# QIR runner (every compiled program is a copy of it plus the program's QIR).
copy_if_exists "${repo_root}/crates/engine/exec/bin" "${output_dir}/crates/engine/exec/bin"
# Developer tools behind `qbnex fmt|lsp|dap`, when a cargo release build made them;
# otherwise the launcher reports how to build them.
for tool in qb-fmt qb-lsp qb-dap; do
  copy_if_exists "${repo_root}/target/release/${tool}" "${output_dir}/${tool}"
done
copy_if_exists "${repo_root}/licenses" "${output_dir}/licenses"
copy_if_exists "${repo_root}/README.md" "${output_dir}/README.md"
copy_if_exists "${repo_root}/CHANGELOG.md" "${output_dir}/CHANGELOG.md"
