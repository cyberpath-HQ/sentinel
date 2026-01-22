#!/usr/bin/env python3
"""
Cross-platform build script for Cyberpath Sentinel C/C++ bindings
"""

import os
import sys
import subprocess
import platform
import shutil
from pathlib import Path

def run_command(cmd, cwd=None, check=True):
    """Run a command and return the result"""
    print(f"Running: {' '.join(cmd)}")
    if cwd:
        print(f"In directory: {cwd}")
    result = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if check and result.returncode != 0:
        print(f"Command failed with return code {result.returncode}")
        print(f"STDOUT: {result.stdout}")
        print(f"STDERR: {result.stderr}")
        sys.exit(1)
    return result

def get_all_target_infos():
    """Get all target platform information from release.yml"""
    return [
        {
            "system": "linux-x86_64",
            "extension": ".so",
            "rust_target": "x86_64-unknown-linux-gnu"
        },
        {
            "system": "macos-x86_64",
            "extension": ".dylib",
            "rust_target": "x86_64-apple-darwin"
        },
        {
            "system": "macos-aarch64",
            "extension": ".dylib",
            "rust_target": "aarch64-apple-darwin"
        },
        {
            "system": "windows-x86_64",
            "extension": ".dll",
            "rust_target": "x86_64-pc-windows-gnu"
        }
    ]

def build_bindings():
    """Build the C/C++ bindings for all targets"""
    script_dir = Path(__file__).parent
    workspace_dir = script_dir / ".." / ".."

    target_infos = get_all_target_infos()

    for target_info in target_infos:
        print(f"\nBuilding for {target_info['system']} ({target_info['rust_target']})")

        # Build from workspace root to ensure proper linking
        cmd = [
            "cargo", "build",
            "--release",
            "--target", target_info['rust_target'],
            "--package", "sentinel-cxx"
        ]

        run_command(cmd, cwd=str(workspace_dir))

        # Find the built libraries in workspace target directory
        target_dir = workspace_dir / "target" / target_info['rust_target'] / "release"
        lib_name = f"libsentinel_cxx{target_info['extension']}"
        static_lib_name = "libsentinel_cxx.a"
        header_name = "sentinel-cxx.h"

        lib_path = target_dir / lib_name
        static_lib_path = target_dir / static_lib_name
        header_path = target_dir / header_name

        if not lib_path.exists() and not static_lib_path.exists():
            raise FileNotFoundError(f"No libraries found in {target_dir}")

        print(f"Built C/C++ bindings for {target_info['system']}")
        if lib_path.exists():
            print(f"Dynamic library: {lib_path}")
        if static_lib_path.exists():
            print(f"Static library: {static_lib_path}")
        if header_path.exists():
            print(f"Header: {header_path}")

if __name__ == "__main__":
    build_bindings()