#!/usr/bin/env python3
"""
API Synchronization Monitor for Cyberpath Sentinel C/C++ Bindings

This script monitors API changes in the core Sentinel library and:
- Detects additions/removals of public functions/structs
- Automatically regenerates C headers via cbindgen
- Provides reports on API changes
- Flags breaking changes requiring manual intervention

NOTE: This script ONLY handles header regeneration and API monitoring.
It does NOT generate C wrapper implementations - those must be added manually.
"""

import os
import sys
import subprocess
import json
import re
from pathlib import Path
from typing import Dict, List, Set, Tuple
from dataclasses import dataclass

@dataclass
class APIChange:
    change_type: str  # 'added', 'removed', 'modified'
    item_type: str    # 'function', 'struct', 'enum', 'trait'
    name: str
    signature: str = ""
    breaking: bool = False

class APISynchronizer:
    """
    Monitors API changes and handles automatic parts of binding updates.

    Capabilities:
    - ✅ Detect API changes (functions, structs, enums, traits)
    - ✅ Regenerate C headers automatically
    - ✅ Copy headers to bindings directory
    - ❌ Generate C wrapper implementations (manual)
    - ❌ Handle complex type conversions (manual)
    """
    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.core_crate = project_root / "crates" / "sentinel"
        self.cxx_crate = project_root / "crates" / "sentinel-cxx"
        self.bindings_dir = project_root / "bindings" / "cxx"

    def run_cargo_doc(self, crate_path: Path) -> Dict:
        """Extract public API from a Rust crate using cargo doc"""
        try:
            result = subprocess.run(
                ["cargo", "doc", "--manifest-path", str(crate_path / "Cargo.toml"), "--no-deps", "--document-private-items"],
                capture_output=True, text=True, cwd=str(crate_path)
            )

            if result.returncode != 0:
                print(f"Warning: Failed to document {crate_path.name}: {result.stderr}")
                return {}

            # Parse the generated documentation
            return self._parse_doc_output(result.stdout)
        except Exception as e:
            print(f"Error documenting {crate_path}: {e}")
            return {}

    def _parse_doc_output(self, output: str) -> Dict:
        """Parse cargo doc output to extract API information"""
        api_info = {
            'functions': [],
            'structs': [],
            'enums': [],
            'traits': []
        }

        # Simple regex-based parsing (could be improved with proper AST parsing)
        lines = output.split('\n')

        current_section = None
        for line in lines:
            if '## Functions' in line:
                current_section = 'functions'
            elif '## Structs' in line:
                current_section = 'structs'
            elif '## Enums' in line:
                current_section = 'enums'
            elif '## Traits' in line:
                current_section = 'traits'
            elif current_section and line.strip().startswith('pub '):
                if current_section in api_info:
                    api_info[current_section].append(line.strip())

        return api_info

    def compare_apis(self, old_api: Dict, new_api: Dict) -> List[APIChange]:
        """Compare two API snapshots and identify changes"""
        changes = []

        for item_type in ['functions', 'structs', 'enums', 'traits']:
            old_items = set(old_api.get(item_type, []))
            new_items = set(new_api.get(item_type, []))

            # Added items
            added = new_items - old_items
            for item in added:
                changes.append(APIChange(
                    change_type='added',
                    item_type=item_type,
                    name=self._extract_name(item),
                    signature=item,
                    breaking=False
                ))

            # Removed items
            removed = old_items - new_items
            for item in removed:
                changes.append(APIChange(
                    change_type='removed',
                    item_type=item_type,
                    name=self._extract_name(item),
                    signature=item,
                    breaking=True
                ))

            # Modified items (signature changes)
            common = old_items & new_items
            for item in common:
                # For now, assume no signature changes (could be enhanced)
                pass

        return changes

    def _extract_name(self, signature: str) -> str:
        """Extract function/struct name from signature"""
        # Simple extraction - could be improved
        match = re.search(r'(?:fn|struct|enum)\s+(\w+)', signature)
        if match:
            return match.group(1)

        # Fallback: extract first word after 'pub'
        parts = signature.split()
        if len(parts) > 1 and parts[0] == 'pub':
            return parts[1]

        return signature.split()[0] if signature.split() else "unknown"

    def update_bindings(self, changes: List[APIChange]):
        """Update C/C++ bindings based on API changes

        This method handles the AUTOMATIC parts:
        - Header regeneration via cbindgen
        - Header file copying
        - Build verification

        It does NOT generate C wrapper implementations.
        """
        print(f"Processing {len(changes)} API changes...")

        breaking_changes = [c for c in changes if c.breaking]
        if breaking_changes:
            print("⚠️  BREAKING CHANGES DETECTED:")
            for change in breaking_changes:
                print(f"  - {change.change_type.upper()} {change.item_type}: {change.name}")
            print("\n❌ Manual intervention required for breaking changes!")
            print("   You need to update C wrapper functions in crates/sentinel-cxx/src/lib.rs")
            return False

        print("🔄 Building C/C++ bindings via cbuild.py...")
        cbuild_script = self.bindings_dir / "cbuild.py"
        result = subprocess.run(
            [sys.executable, str(cbuild_script)],
            cwd=str(self.project_root),
            capture_output=True, text=True
        )

        if result.returncode != 0:
            print(f"❌ Error building bindings: {result.stderr}")
            return False

        # Copy libraries to bindings/cxx/lib
        self._copy_libraries()

        # Copy updated headers
        self._copy_headers()

        print("✅ Headers updated automatically")
        print("⚠️  REMINDER: C wrapper implementations must be added manually!")
        return True

    def _copy_headers(self):
        """Copy generated headers to bindings directory"""
        header_src = self.cxx_crate / "target" / "release" / "sentinel-cxx.h"
        header_dst = self.bindings_dir / "include" / "sentinel" / "sentinel-cxx.h"

        if header_src.exists():
            header_dst.parent.mkdir(parents=True, exist_ok=True)
            import shutil
            shutil.copy2(header_src, header_dst)
            print(f"Copied header: {header_src} -> {header_dst}")

    def _copy_libraries(self):
        """Copy built libraries to bindings/cxx/lib directory"""
        import platform
        import shutil

        system = platform.system().lower()
        machine = platform.machine().lower()

        if system == "windows":
            lib_name = "sentinel_cxx.dll"
            rust_target = "x86_64-pc-windows-msvc" if machine == "amd64" else f"{machine}-pc-windows-msvc"
        elif system == "darwin":
            lib_name = "libsentinel_cxx.dylib"
            rust_target = "x86_64-apple-darwin" if machine == "x86_64" else "aarch64-apple-darwin"
        elif system == "linux":
            lib_name = "libsentinel_cxx.so"
            rust_target = f"{machine}-unknown-linux-gnu"
        else:
            print(f"Warning: Unsupported platform {system}, skipping library copy")
            return

        # Source library from cbuild.py output
        lib_src = self.bindings_dir / "dist" / lib_name

        # Destination in bindings/cxx/lib
        lib_dst = self.bindings_dir / "lib" / lib_name

        if lib_src.exists():
            lib_dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(lib_src, lib_dst)
            print(f"Copied library: {lib_src} -> {lib_dst}")
        else:
            print(f"Warning: Built library not found at {lib_src}")

    def generate_report(self, changes: List[APIChange]) -> str:
        """Generate a report of API changes"""
        report = ["# API Synchronization Report\n"]

        if not changes:
            report.append("✅ No API changes detected.")
            return "\n".join(report)

        report.append(f"## Summary: {len(changes)} changes\n")

        breaking = [c for c in changes if c.breaking]
        non_breaking = [c for c in changes if not c.breaking]

        if breaking:
            report.append(f"⚠️  **Breaking Changes: {len(breaking)}**")
            for change in breaking:
                report.append(f"- {change.change_type.upper()} {change.item_type}: `{change.name}`")
            report.append("")

        if non_breaking:
            report.append(f"✅ **Non-Breaking Changes: {len(non_breaking)}**")
            for change in non_breaking:
                report.append(f"- {change.change_type.upper()} {change.item_type}: `{change.name}`")
            report.append("")

        report.append("## Actions Taken")
        if breaking:
            report.append("- Manual review required for breaking changes")
            report.append("- Bindings NOT automatically updated")
            report.append("- C wrapper implementations must be added manually")
        else:
            report.append("- Headers automatically regenerated via cbindgen")
            report.append("- Headers updated in bindings directory")
            report.append("- **C wrapper implementations still need manual addition**")

        return "\n".join(report)

    def save_baseline(self, api_snapshot: Dict, filename: str = "api_baseline.json"):
        """Save current API as baseline for future comparisons"""
        baseline_file = self.bindings_dir / filename
        baseline_file.parent.mkdir(parents=True, exist_ok=True)

        with open(baseline_file, 'w') as f:
            json.dump(api_snapshot, f, indent=2)

    def load_baseline(self, filename: str = "api_baseline.json") -> Dict:
        """Load previous API baseline"""
        baseline_file = self.bindings_dir / filename
        if not baseline_file.exists():
            return {}

        try:
            with open(baseline_file, 'r') as f:
                return json.load(f)
        except Exception:
            return {}

def main():
    if len(sys.argv) != 2:
        print("Usage: python api_sync.py <command>")
        print("Commands: check, update, baseline")
        sys.exit(1)

    command = sys.argv[1]
    project_root = Path(__file__).parent.parent.parent

    sync = APISynchronizer(project_root)

    if command == "baseline":
        # Create baseline API snapshot
        print("Creating API baseline...")
        api = sync.run_cargo_doc(sync.core_crate)
        sync.save_baseline(api)
        print("✓ Baseline created")

    elif command == "check":
        # Check for API changes
        print("Checking for API changes...")
        current_api = sync.run_cargo_doc(sync.core_crate)
        baseline_api = sync.load_baseline()

        if not baseline_api:
            print("No baseline found. Run 'baseline' command first.")
            sys.exit(1)

        changes = sync.compare_apis(baseline_api, current_api)
        report = sync.generate_report(changes)
        print(report)

        if changes:
            # Save report
            report_file = sync.bindings_dir / "api_changes.md"
            with open(report_file, 'w') as f:
                f.write(report)
            print(f"Report saved to: {report_file}")

    elif command == "update":
        # Update bindings automatically
        print("Updating bindings...")
        current_api = sync.run_cargo_doc(sync.core_crate)
        baseline_api = sync.load_baseline()

        if not baseline_api:
            print("No baseline found. Run 'baseline' command first.")
            sys.exit(1)

        changes = sync.compare_apis(baseline_api, current_api)

        if sync.update_bindings(changes):
            # Update baseline
            sync.save_baseline(current_api)
            print("✓ Bindings updated and baseline refreshed")
        else:
            print("✗ Bindings update failed - manual intervention required")
            sys.exit(1)

    else:
        print(f"Unknown command: {command}")
        sys.exit(1)

if __name__ == "__main__":
    main()