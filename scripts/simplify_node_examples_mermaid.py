#!/usr/bin/env python3
"""Remove mermaid feature flag from node examples: always use .mmd, remove graph! fallback."""
import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CARGO = REPO / "Cargo.toml"


def get_node_example_paths():
    paths = []
    with open(CARGO) as f:
        for line in f:
            m = re.match(r'^\s*path\s*=\s*"(examples/nodes/[^"]+\.rs)"', line)
            if m:
                paths.append(m.group(1))
    return paths


def simplify(content: str) -> str:
    # 1. Remove gated use graph; keep Graph (we need it for the type)
    content = re.sub(
        r'#\[cfg\(not\(feature = "mermaid"\)\)\]\s*\n\s*use streamweave::graph;\s*\n',
        "",
        content,
        flags=re.MULTILINE,
    )
    content = re.sub(
        r'#\[cfg\(not\(feature = "mermaid"\)\)\]\s*\n\s*use streamweave::graph::Graph;\s*\n',
        "use streamweave::graph::Graph;\n",
        content,
        flags=re.MULTILINE,
    )
    # 2. Un-gate the mermaid block (keep newline before "let mut graph")
    content = re.sub(
        r'\s*#\[cfg\(feature = "mermaid"\)\]\s*\n\s*let mut graph: streamweave::graph::Graph = \{',
        "\n  let mut graph: streamweave::graph::Graph = {",
        content,
        count=1,
    )
    # 3. Remove the #[cfg(not(feature = "mermaid"))] block (graph! fallback)
    start_marker = '#[cfg(not(feature = "mermaid"))]'
    idx = content.find(start_marker)
    if idx == -1:
        return content
    rest = content[idx:]
    lines = rest.split("\n")
    depth = 0
    in_block = False
    end_line_i = 0
    for i, line in enumerate(lines):
        if "let mut graph: Graph = graph!" in line:
            in_block = True
            depth = line.count("{") - line.count("}")
            continue
        if in_block:
            depth += line.count("{") - line.count("}")
            if depth == 0 and "};" in line:
                end_line_i = i
                break
    else:
        return content
    end_pos_in_rest = sum(len(l) + 1 for l in lines[: end_line_i + 1])
    content = content[:idx].rstrip() + "\n" + content[idx + end_pos_in_rest :].lstrip()
    return content


def main():
    paths = get_node_example_paths()
    for rs_path in paths:
        full = REPO / rs_path
        if not full.exists():
            continue
        content = full.read_text()
        if 'parse_mmd_file_to_blueprint' not in content or 'cfg(feature = "mermaid")' not in content:
            continue
        new_content = simplify(content)
        if new_content != content:
            full.write_text(new_content)
            print(rs_path)


if __name__ == "__main__":
    main()
