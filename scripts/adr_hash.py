#!/usr/bin/env python3
import argparse
import hashlib
import re
from pathlib import Path


ADR_LINE_RE = re.compile(r"^(ADR_BODY_SHA256:)\s*([0-9a-fA-F]{64})\s*$")


def normalized_for_hash(text: str) -> str:
    # Normalize line endings and replace the hash value with 64 zeroes so the hash is stable.
    lines = text.replace("\r\n", "\n").replace("\r", "\n").split("\n")
    out = []
    replaced = 0
    for line in lines:
        m = ADR_LINE_RE.match(line)
        if m:
            out.append(f"{m.group(1)} " + ("0" * 64))
            replaced += 1
        else:
            out.append(line)
    if replaced != 1:
        raise SystemExit(
            f"expected exactly one ADR_BODY_SHA256 line, found {replaced}; refusing to hash"
        )
    return "\n".join(out)


def compute_hash(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    normalized = normalized_for_hash(text).encode("utf-8")
    return hashlib.sha256(normalized).hexdigest()


def fix_file(path: Path) -> str:
    text = path.read_text(encoding="utf-8").replace("\r\n", "\n").replace("\r", "\n")
    new_hash = compute_hash(path)

    lines = text.split("\n")
    replaced = 0
    for i, line in enumerate(lines):
        m = ADR_LINE_RE.match(line)
        if m:
            lines[i] = f"{m.group(1)} {new_hash}"
            replaced += 1
    if replaced != 1:
        raise SystemExit(
            f"expected exactly one ADR_BODY_SHA256 line, found {replaced}; refusing to fix"
        )
    path.write_text("\n".join(lines), encoding="utf-8")
    return new_hash


def main() -> int:
    ap = argparse.ArgumentParser(description="Check/fix ADR_BODY_SHA256 drift guard.")
    ap.add_argument("adr", type=Path, help="Path to ADR markdown file")
    ap.add_argument("--fix", action="store_true", help="Rewrite ADR_BODY_SHA256 in-place")
    args = ap.parse_args()

    if not args.adr.exists():
        raise SystemExit(f"no such file: {args.adr}")

    if args.fix:
        h = fix_file(args.adr)
        print(h)
        return 0

    expected = compute_hash(args.adr)
    text = args.adr.read_text(encoding="utf-8").replace("\r\n", "\n").replace("\r", "\n")
    actual = None
    for line in text.split("\n"):
        m = ADR_LINE_RE.match(line)
        if m:
            actual = m.group(2).lower()
            break

    if actual is None:
        raise SystemExit("missing ADR_BODY_SHA256 line")

    if actual != expected:
        print(f"ADR hash mismatch:\n  file:     {actual}\n  expected: {expected}")
        return 1

    print(expected)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

