#!/usr/bin/env python3
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent
PREFIXES = ("br", "cr", "b", "c", "r")
IDENT = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_")


def string_end(text, i):
    while i < len(text):
        if text[i] == "\\":
            i += 2
        elif text[i] == '"':
            return i + 1
        else:
            i += 1
    return i


def raw_string_end(text, i, hashes):
    closing = '"' + "#" * hashes
    found = text.find(closing, i)
    return len(text) if found < 0 else found + len(closing)


def block_end(text, i):
    depth = 1
    while i < len(text) and depth:
        if text.startswith("/*", i):
            depth, i = depth + 1, i + 2
        elif text.startswith("*/", i):
            depth, i = depth - 1, i + 2
        else:
            i += 1
    return i


def raw_prefix(text, i):
    j = i + 1
    while j < len(text) and text[j] == "#":
        j += 1
    if j < len(text) and text[j] == '"':
        return j - i - 1, j + 1
    return None


def comments(text):
    i = 0
    while i < len(text):
        c = text[i]
        previous = text[i - 1] if i else ""
        if text.startswith("//", i):
            doc = (text.startswith("///", i) and not text.startswith("////", i)) or text.startswith("//!", i)
            if not doc:
                yield i
            end = text.find("\n", i)
            i = len(text) if end < 0 else end
        elif text.startswith("/*", i):
            doc = (
                text.startswith("/**", i)
                and not text.startswith("/***", i)
                and not text.startswith("/**/", i)
            ) or text.startswith("/*!", i)
            if not doc:
                yield i
            i = block_end(text, i + 2)
        elif c == '"':
            i = string_end(text, i + 1)
        elif c in "bcr" and previous not in IDENT:
            prefix = next((p for p in PREFIXES if text.startswith(p, i)), "")
            rest = i + len(prefix)
            raw = raw_prefix(text, rest - 1) if prefix.endswith("r") else None
            if raw:
                i = raw_string_end(text, raw[1], raw[0])
            elif text.startswith('"', rest):
                i = string_end(text, rest + 1)
            elif prefix == "b" and text.startswith("'", rest):
                i = char_end(text, rest)
            else:
                i += 1
        elif c == "'":
            i = char_end(text, i)
        else:
            i += 1


def char_end(text, i):
    if text.startswith("'\\", i):
        end = text.find("'", i + 3)
        return len(text) if end < 0 else end + 1
    if i + 2 < len(text) and text[i + 2] == "'":
        return i + 3
    return i + 1


def main():
    found = []
    for directory in ("src", "tests", "examples"):
        for path in sorted((ROOT / directory).rglob("*.rs")):
            text = path.read_text(encoding="utf-8")
            for offset in comments(text):
                found.append(f"{path.relative_to(ROOT)}:{text.count(chr(10), 0, offset) + 1}: a comment that is not a doc comment")
    for line in found:
        print(line)
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
