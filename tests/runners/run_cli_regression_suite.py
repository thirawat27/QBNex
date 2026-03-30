#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import re
import signal
import subprocess
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Shard and run the shell_cli regression suite with per-test timeouts."
    )
    parser.add_argument("--workspace", default="D:\\QBNex", help="Workspace root")
    parser.add_argument(
        "--timeout-seconds", type=int, default=90, help="Per-test timeout in seconds"
    )
    parser.add_argument("--filter", default="", help="Run only tests whose names contain this text")
    parser.add_argument(
        "--include-ignored",
        action="store_true",
        help="Include ignored tests by passing --ignored to the test binary",
    )
    return parser.parse_args()


def run_checked(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command,
        cwd=cwd,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed: {' '.join(command)}\nstdout:\n{completed.stdout}\nstderr:\n{completed.stderr}"
        )
    return completed


def discover_shell_cli_binary(workspace: Path) -> Path:
    command = [
        "cargo",
        "test",
        "-p",
        "cli_tool",
        "--test",
        "shell_cli",
        "--no-run",
        "--message-format=json",
    ]
    completed = run_checked(command, workspace)
    executable: Path | None = None
    for line in completed.stdout.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("reason") != "compiler-artifact":
            continue
        target = message.get("target", {})
        if target.get("name") != "shell_cli":
            continue
        candidate = message.get("executable")
        if candidate:
            executable = Path(candidate)
    if executable is None or not executable.exists():
        raise RuntimeError("could not locate shell_cli test binary from cargo output")
    return executable


def list_shell_cli_tests(binary: Path, name_filter: str) -> list[str]:
    completed = run_checked([str(binary), "--list"], binary.parent)
    tests: list[str] = []
    for line in completed.stdout.splitlines():
        match = re.match(r"^(.*): test$", line.strip())
        if not match:
            continue
        name = match.group(1).strip()
        if not name_filter or name_filter in name:
            tests.append(name)
    return tests


def terminate_process_tree(process: subprocess.Popen[str]) -> None:
    if process.poll() is not None:
        return
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/F", "/T", "/PID", str(process.pid)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    else:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def run_one_test(binary: Path, workspace: Path, test_name: str, timeout: int, include_ignored: bool) -> tuple[str, str]:
    args = [str(binary)]
    if include_ignored:
        args.append("--ignored")
    args.extend(["--exact", test_name, "--nocapture"])
    popen_kwargs: dict[str, object] = {
        "cwd": workspace,
        "stdout": subprocess.PIPE,
        "stderr": subprocess.PIPE,
        "text": True,
    }
    if os.name == "nt":
        popen_kwargs["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP  # type: ignore[attr-defined]
    else:
        popen_kwargs["start_new_session"] = True

    process = subprocess.Popen(args, **popen_kwargs)
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        terminate_process_tree(process)
        try:
            stdout, stderr = process.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            stdout, stderr = "", ""
        return "timeout", stdout + stderr

    combined = stdout + stderr
    if "... ignored" in combined:
        return "ignored", combined
    if process.returncode == 0:
        return "passed", combined
    return "failed", combined


def main() -> int:
    args = parse_args()
    workspace = Path(args.workspace).resolve()
    binary = discover_shell_cli_binary(workspace)
    tests = list_shell_cli_tests(binary, args.filter)
    if not tests:
        raise RuntimeError("no shell_cli tests matched the current filter")

    print(f"shell_cli binary: {binary}")
    print(f"tests selected: {len(tests)}")
    print(f"timeout per test: {args.timeout_seconds}s")
    if args.include_ignored:
        print("mode: include ignored tests")

    passed = 0
    ignored = 0
    failed: list[tuple[str, str]] = []
    timed_out: list[tuple[str, str]] = []

    for index, test_name in enumerate(tests, start=1):
        print(f"[{index}/{len(tests)}] {test_name}")
        status, output = run_one_test(
            binary,
            workspace,
            test_name,
            args.timeout_seconds,
            args.include_ignored,
        )
        if status == "passed":
            passed += 1
            print("  PASS")
        elif status == "ignored":
            ignored += 1
            print("  IGNORED")
        elif status == "timeout":
            timed_out.append((test_name, output))
            print("  TIMEOUT")
        else:
            failed.append((test_name, output))
            print("  FAIL")

    print(
        f"summary: passed={passed} ignored={ignored} failed={len(failed)} timed_out={len(timed_out)}"
    )

    if failed:
        print("\nfailed tests:", file=sys.stderr)
        for name, output in failed:
            print(f"\n--- {name} ---\n{output}", file=sys.stderr)

    if timed_out:
        print("\ntimed out tests:", file=sys.stderr)
        for name, output in timed_out:
            print(f"\n--- {name} ---\n{output}", file=sys.stderr)

    return 1 if failed or timed_out else 0


if __name__ == "__main__":
    sys.exit(main())
