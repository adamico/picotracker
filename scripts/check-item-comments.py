#!/usr/bin/env python3
"""Fails on comments attached to constants, statics, enums, enum variants and struct fields.

Their names carry the meaning: a name that needs a comment needs a better name instead.
Comments on modules, functions, impls and struct types are untouched.

Usage: scripts/check-item-comments.py [folder ...]   (default: crates)
"""

import pathlib
import re
import sys

CONSTANT = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?(const|static)\s+(mut\s+)?[A-Z_][A-Z0-9_]*\s*:")
ENUM = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?enum\s+\w+")
STRUCT = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?struct\s+\w+")


def is_comment(line: str) -> bool:
    stripped = line.strip()
    return stripped.startswith("//") and not stripped.startswith("//!")


def trailing_comment(line: str) -> bool:
    code = re.sub(r'"(\\.|[^"\\])*"', '""', line)
    return "//" in code and not code.strip().startswith("//")


def body_end(lines: list[str], start: int) -> int:
    """The index of the line that closes the braced body opening on `start`."""
    depth = 0
    for index in range(start, len(lines)):
        code = re.sub(r'"(\\.|[^"\\])*"', '""', lines[index].split("//")[0])
        depth += code.count("{") - code.count("}")
        if depth <= 0 and index > start or (index == start and depth == 0 and "{" in code):
            return index
    return len(lines) - 1


def check(path: pathlib.Path) -> list[str]:
    lines = path.read_text(encoding="utf-8").splitlines()
    problems = []

    def report(index: int, what: str) -> None:
        problems.append(f"{path}:{index + 1}: comment on {what}")

    def comments_above(index: int) -> list[int]:
        found = []
        above = index - 1
        while above >= 0 and (is_comment(lines[above]) or lines[above].strip().startswith("#[")):
            if is_comment(lines[above]):
                found.append(above)
            above -= 1
        return found

    for index, line in enumerate(lines):
        if CONSTANT.match(line):
            what = "a constant"
        elif ENUM.match(line):
            what = "an enum"
        else:
            what = None
        if what:
            for above in comments_above(index):
                report(above, what)
            if trailing_comment(line):
                report(index, what)

        kind = "enum" if ENUM.match(line) else "struct" if STRUCT.match(line) else None
        if kind and line.rstrip().endswith("{"):
            member = "an enum variant" if kind == "enum" else "a struct field"
            for inside in range(index + 1, body_end(lines, index)):
                if is_comment(lines[inside]) or trailing_comment(lines[inside]):
                    report(inside, member)
    return problems


def main() -> int:
    folders = [pathlib.Path(arg) for arg in sys.argv[1:]] or [pathlib.Path("crates")]
    problems = []
    for folder in folders:
        for path in sorted(folder.rglob("*.rs")):
            if "target" not in path.parts:
                problems.extend(check(path))
    for problem in problems:
        print(problem)
    if problems:
        print(
            f"\n{len(problems)} comment(s) on constants, enums or struct fields. "
            "Let the name carry the meaning: delete the comment, or rename the item.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
