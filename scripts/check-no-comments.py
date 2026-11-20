#!/usr/bin/env python3
"""Fails on any comment in Rust sources: line, block, doc and module comments alike.

Code carries no comments. A name, a type or a test says what code does, and an ADR says why.

Usage: scripts/check-no-comments.py [folder ...]   (default: crates)
"""

import pathlib
import sys


def comment_lines(text: str) -> list[int]:
    found = []
    line = 1
    index = 0
    length = len(text)

    def advance_to(end: int) -> None:
        nonlocal index, line
        line += text.count("\n", index, end)
        index = end

    while index < length:
        char = text[index]
        pair = text[index : index + 2]
        if pair == "//":
            found.append(line)
            end = text.find("\n", index)
            advance_to(length if end < 0 else end)
        elif pair == "/*":
            found.append(line)
            depth = 0
            cursor = index
            while cursor < length:
                if text.startswith("/*", cursor):
                    depth += 1
                    cursor += 2
                elif text.startswith("*/", cursor):
                    depth -= 1
                    cursor += 2
                    if depth == 0:
                        break
                else:
                    cursor += 1
            advance_to(cursor)
        elif char == "r" or (char == "b" and text[index + 1 : index + 2] == "r"):
            start = index + (2 if char == "b" else 1)
            hashes = 0
            while text[start + hashes : start + hashes + 1] == "#":
                hashes += 1
            previous = text[index - 1] if index else " "
            if text[start + hashes : start + hashes + 1] == '"' and not (previous.isalnum() or previous == "_"):
                closing = '"' + "#" * hashes
                end = text.find(closing, start + hashes + 1)
                advance_to(length if end < 0 else end + len(closing))
            else:
                advance_to(index + 1)
        elif char == '"':
            cursor = index + 1
            while cursor < length and text[cursor] != '"':
                cursor += 2 if text[cursor] == "\\" else 1
            advance_to(cursor + 1)
        elif char == "'":
            if text[index + 1 : index + 2] == "\\":
                end = text.find("'", index + 3)
                advance_to(end + 1)
            elif text[index + 2 : index + 3] == "'":
                advance_to(index + 3)
            else:
                advance_to(index + 1)
        else:
            advance_to(index + 1)
    return found


def main() -> int:
    folders = [pathlib.Path(arg) for arg in sys.argv[1:]] or [pathlib.Path("crates")]
    problems = []
    for folder in folders:
        paths = [folder] if folder.is_file() else sorted(folder.rglob("*.rs"))
        for path in paths:
            if "target" not in path.parts:
                for line in comment_lines(path.read_text(encoding="utf-8")):
                    problems.append(f"{path}:{line}: comment")
    for problem in problems:
        print(problem)
    if problems:
        print(
            f"\n{len(problems)} comment(s). Let a name, a type or a test carry the meaning, "
            "and record the reason behind a choice in an ADR under docs/adr/.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
