"""Render LLVM branch counters and reject missing instrumentation."""
import json
from pathlib import Path
import sys


def counts(summary):
    branches = summary.get("branches", {})
    covered, total = branches.get("covered", 0), branches.get("count", 0)
    if not 0 <= covered <= total:
        raise ValueError("Invalid LLVM branch counters")
    return covered, total


def row(name, covered, total):
    ratio = f"{100 * covered / total:.2f}%" if total else "—"
    print(f"| {name} | {covered} | {total} | {ratio} |")


def main():
    export = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))["data"]
    if len(export) != 1:
        raise ValueError("Expected one merged LLVM coverage export")
    data = export[0]
    covered, total = counts(data["totals"])
    parser_has_branches = any(
        Path(file["filename"]).as_posix().endswith("/src/expression.rs")
        and counts(file["summary"])[1] > 0
        for file in data["files"]
    )
    if total == 0 or not parser_has_branches:
        raise ValueError("Expected measured LLVM branch counters, including parser branches")
    print("| File | Covered | Total | Branch coverage |")
    print("| --- | ---: | ---: | ---: |")
    for file in data["files"]:
        filename = Path(file["filename"]).as_posix()
        marker = filename.rfind("/src/")
        display = filename[marker + 1:] if marker >= 0 else Path(filename).name
        row(display, *counts(file["summary"]))
    row("**Total**", covered, total)


if __name__ == "__main__":
    main()
