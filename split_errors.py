#!/usr/bin/env python3
"""
Split log/check.log into individual error files.

Each error from the Rust compiler output is extracted and saved to
log/errors/<number>.error where <number> is the error index (1-indexed).
"""

import re
import os
from pathlib import Path


def extract_errors(log_file_path: str, output_dir: str) -> int:
    """
    Extract errors from the log file and save them to individual files.
    
    Args:
        log_file_path: Path to the input log file
        output_dir: Directory to save error files to
        
    Returns:
        Number of errors extracted
    """
    # Create output directory if it doesn't exist
    Path(output_dir).mkdir(parents=True, exist_ok=True)
    
    # Pattern to match error start lines - look for "error[E" or "error:" anywhere in the line
    # This handles ANSI escape codes by looking for the actual text
    error_start_pattern = re.compile(r'error\[E\d+\]|error\[0m\[0m\[1m:')
    
    errors = []
    current_error = []
    
    with open(log_file_path, 'r', encoding='utf-8', errors='ignore') as f:
        for line in f:
            # Check if this line starts a new error
            if error_start_pattern.search(line):
                # Save previous error if exists
                if current_error:
                    errors.append(''.join(current_error))
                
                # Start new error
                current_error = [line]
            elif current_error:
                # Continue current error
                # Stop at blank lines that might separate errors, but check if next line is also blank
                # or starts a new error. For now, include all lines until next error.
                current_error.append(line)
    
        # Don't forget the last error
        if current_error:
            errors.append(''.join(current_error))
    
    # Clean up: Remove trailing blank lines from each error
    cleaned_errors = []
    for error in errors:
        # Remove trailing newlines but keep at least one at the end
        cleaned = error.rstrip() + '\n'
        if cleaned.strip():  # Only add if not empty
            cleaned_errors.append(cleaned)
    
    # Write each error to a separate file
    for i, error_text in enumerate(cleaned_errors, start=1):
        error_file = os.path.join(output_dir, f"{i}.error")
        with open(error_file, 'w', encoding='utf-8') as f:
            f.write(error_text)
    
    return len(cleaned_errors)


def main():
    log_file = "log/check.log"
    output_dir = "log/errors"
    
    if not os.path.exists(log_file):
        print(f"Error: {log_file} not found")
        return 1
    
    print(f"Reading errors from {log_file}...")
    error_count = extract_errors(log_file, output_dir)
    print(f"Extracted {error_count} errors to {output_dir}/")
    
    return 0


if __name__ == "__main__":
    exit(main())
