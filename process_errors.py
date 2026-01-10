#!/usr/bin/env python3
"""
Process error files sequentially using opencode.

This script:
1. Lists files in log/errors/ directory
2. Takes the first filename
3. Runs opencode to fix the problem described in that file
4. Deletes the file
5. Repeats until no files remain
"""

import os
import subprocess
import sys
from pathlib import Path


def get_first_error_file(error_dir: str) -> str | None:
    """
    Get the first error file from the directory, sorted numerically.

    Args:
        error_dir: Directory containing error files

    Returns:
        Path to the first error file, or None if no files exist
    """
    error_path = Path(error_dir)

    if not error_path.exists():
        print(f"Error directory {error_dir} does not exist")
        return None

    # Get all .error files and sort them numerically
    error_files = sorted(
        error_path.glob("*.error"),
        key=lambda x: int(x.stem) if x.stem.isdigit() else float("inf"),
    )

    if not error_files:
        return None

    return str(error_files[0])


def run_opencode_fix(filepath: str) -> bool:
    """
    Run opencode to fix the problem described in the file.

    Args:
        filepath: Path to the error file

    Returns:
        True if command succeeded, False otherwise
    """
    # Construct the command as specified by the user
    # Format: nix-shell -p opencode --run "opencode run \"Fix the problem described in <filepath>\""
    # Note: Using absolute path to ensure opencode can find the file
    abs_filepath = os.path.abspath(filepath)

    # The outer command wraps the inner command in quotes
    shell_cmd = f"nix-shell -p opencode --run 'opencode run \"Fix the problem described in {abs_filepath}\". Don\t run any tools; just fix the code once and exit.'"

    print(f"Running: {shell_cmd}")
    print(f"Processing file: {abs_filepath}")

    try:
        # Run the command using shell=True to handle nested quotes properly
        result = subprocess.run(
            shell_cmd,
            shell=True,
            check=False,  # Don't raise on error, we'll handle it
            capture_output=True,
            text=True,
            cwd=os.getcwd(),
        )

        # Print output
        if result.stdout:
            print(result.stdout)
        if result.stderr:
            print(result.stderr, file=sys.stderr)

        if result.returncode != 0:
            print(f"Warning: Command exited with code {result.returncode}")
            return False

        return True
    except Exception as e:
        print(f"Error running opencode: {e}")
        return False


def delete_file(filepath: str) -> bool:
    """
    Delete the error file.

    Args:
        filepath: Path to the file to delete

    Returns:
        True if deleted successfully, False otherwise
    """
    try:
        os.remove(filepath)
        print(f"Deleted: {filepath}")
        return True
    except Exception as e:
        print(f"Error deleting file {filepath}: {e}")
        return False


def main():
    """Main loop to process all error files."""
    error_dir = "log/errors"
    processed_count = 0

    print(f"Starting to process error files in {error_dir}/")
    print("Press Ctrl+C to stop\n")

    try:
        while True:
            # Get the first error file
            error_file = get_first_error_file(error_dir)

            if error_file is None:
                print(
                    f"\nNo more error files to process. Total processed: {processed_count}"
                )
                break

            print(f"\n--- Processing file {processed_count + 1} ---")

            # Run opencode to fix the problem
            success = run_opencode_fix(error_file)

            # Delete the file regardless of success/failure
            # (you may want to change this behavior)
            deleted = delete_file(error_file)

            if deleted:
                processed_count += 1

            # Small delay to avoid overwhelming the system
            # Remove or adjust if not needed
            # time.sleep(0.1)

    except KeyboardInterrupt:
        print(f"\n\nInterrupted by user. Total processed: {processed_count}")
        return 1
    except Exception as e:
        print(f"\nUnexpected error: {e}")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
