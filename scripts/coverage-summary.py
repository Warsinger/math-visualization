"""Turn LLVM's report into a per-source-file Actions summary; fail on empty data."""
import json
import sys
from pathlib import Path

report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))["data"][0]
files = report["files"]
if not files or report["totals"]["lines"]["count"] == 0:
    raise SystemExit("Coverage report contains no measured source lines")

print("## Coverage: Linux x64, stable Rust, debug profile\n")
print("One instrumented run of all automated test targets. No percentage gate. "
      "Branch coverage, other platforms/profiles, native GUI interaction and pixels are unmeasured. "
      "Dependencies, generated target files and integration-test source are excluded; "
      "inline unit-test code remains included.\n")
print("Download **coverage-linux-debug** and open **html/index.html** for uncovered source locations.\n")
print("| Source | Lines | Functions | Regions |")
print("| --- | ---: | ---: | ---: |")

def row(name, summary):
    values = [f"{summary[k]['percent']:.2f}% ({summary[k]['covered']}/{summary[k]['count']})"
              for k in ("lines", "functions", "regions")]
    print("| " + name + " | " + " | ".join(values) + " |")

for file in sorted(files, key=lambda item: item["filename"]):
    name = file["filename"].replace("\\", "/").split("/src/")[-1]
    row(name.replace("|", "\\|"), file["summary"])
row("**Total**", report["totals"])
