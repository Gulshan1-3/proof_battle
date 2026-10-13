#!/usr/bin/env python3
"""
Documentation Link and Integrity Linter for ProofBattle.

Scans all Markdown (.md) documents in the repository:
1. Validates that local relative file links exist on disk.
2. Validates that markdown anchors (#heading-name) resolve to actual headings.
3. Reports broken links with file path and line number.
Exits 0 on success, 1 on broken links.
"""

import os
import re
import sys
from pathlib import Path
from typing import List, Tuple, Set, Dict

ROOT_DIR = Path(__file__).resolve().parent.parent.parent

EXCLUDE_DIRS = {
    ".git",
    "node_modules",
    "target",
    ".lake",
    ".svelte-kit",
    "dist",
    "build",
    "brain",
    "infra",
    "lean",
    "data"
}

def slugify_headings(heading: str) -> Set[str]:
    """Generates standard GitHub anchor slug variations for a markdown heading."""
    # Remove markdown link syntax: [text](link) -> text
    heading = re.sub(r'\[([^\]]+)\]\([^\)]+\)', r'\1', heading)
    # Remove inline code ticks, bold, italics
    heading = re.sub(r'[`*_]', '', heading)
    raw = heading.strip().lower()

    slugs = set()

    # Variation 1: Strict GitHub (punctuation to empty, single hyphen collapse)
    s1 = re.sub(r'[^\w\s-]', '', raw)
    s1 = re.sub(r'\s+', '-', s1)
    slugs.add(s1)

    # Variation 2: Punctuation replaced by hyphens (common in TOC generators for &, em-dash, etc.)
    s2 = re.sub(r'[^\w-]', '-', raw)
    slugs.add(s2)
    s2_collapsed = re.sub(r'-+', '-', s2).strip('-')
    slugs.add(s2_collapsed)

    # Variation 3: Double hyphen preservation (e.g., "A & B" -> "a--b")
    s3 = re.sub(r'\s+[^\w\s-]+\s+', '--', raw)
    s3 = re.sub(r'[^\w\s-]', '', s3)
    s3 = re.sub(r'\s+', '-', s3)
    slugs.add(s3)

    return slugs

def extract_headings(file_path: Path) -> Set[str]:
    """Extracts all anchor slugs from markdown headings in a file."""
    anchors = set()
    try:
        content = file_path.read_text(encoding="utf-8", errors="ignore")
    except Exception:
        return anchors

    for line in content.splitlines():
        line = line.strip()
        m = re.match(r'^(#{1,6})\s+(.+)$', line)
        if m:
            heading_text = m.group(2)
            anchors.update(slugify_headings(heading_text))
    return anchors

def find_markdown_files() -> List[Path]:
    """Finds all project markdown files excluding vendor/upstream dependencies."""
    md_files = []
    for root, dirs, files in os.walk(ROOT_DIR):
        dirs[:] = [d for d in dirs if d not in EXCLUDE_DIRS and not d.startswith(".")]
        for f in files:
            if f.endswith(".md"):
                md_files.append(Path(root) / f)
    return sorted(md_files)

def lint_file(file_path: Path, headings_cache: Dict[Path, Set[str]]) -> List[str]:
    """Checks all links in a single markdown file."""
    errors = []
    try:
        content = file_path.read_text(encoding="utf-8", errors="ignore")
    except Exception as e:
        return [f"Could not read {file_path}: {e}"]

    # Pattern for markdown links: [text](target)
    link_pattern = re.compile(r'\[([^\]]+)\]\(([^)]+)\)')

    for line_idx, line in enumerate(content.splitlines(), start=1):
        for match in link_pattern.finditer(line):
            link_text, target = match.group(1), match.group(2).strip()

            # Ignore external web links, mailto, etc.
            if target.startswith(("http://", "https://", "mailto:", "javascript:")):
                continue

            # Strip file:// scheme if present
            if target.startswith("file://"):
                target = target[len("file://"):]

            # Split path and anchor
            if "#" in target:
                target_path_str, anchor = target.split("#", 1)
            else:
                target_path_str, anchor = target, None

            # Check anchor-only links in the same file
            if not target_path_str:
                if anchor:
                    if file_path not in headings_cache:
                        headings_cache[file_path] = extract_headings(file_path)
                    if anchor.lower() not in headings_cache[file_path]:
                        errors.append(
                            f"{file_path.relative_to(ROOT_DIR)}:{line_idx}: Anchor '#{anchor}' not found in file"
                        )
                continue

            # Resolve target path
            if target_path_str.startswith("/") and "/proof_battle/" in target_path_str:
                target_path_str = target_path_str.split("/proof_battle/", 1)[1]
                resolved_target = (ROOT_DIR / target_path_str).resolve()
            else:
                resolved_target = (file_path.parent / target_path_str).resolve()

            # Skip checking files inside excluded / gitignored directories (e.g. data, infra, lean)
            try:
                rel = resolved_target.relative_to(ROOT_DIR)
                if any(part in EXCLUDE_DIRS for part in rel.parts):
                    continue
            except ValueError:
                pass

            if not resolved_target.exists():
                errors.append(
                    f"{file_path.relative_to(ROOT_DIR)}:{line_idx}: Target file '{target_path_str}' does not exist (resolved to {resolved_target})"
                )
            elif anchor:
                if resolved_target.is_file() and resolved_target.suffix == ".md":
                    if resolved_target not in headings_cache:
                        headings_cache[resolved_target] = extract_headings(resolved_target)
                    if anchor.lower() not in headings_cache[resolved_target]:
                        errors.append(
                            f"{file_path.relative_to(ROOT_DIR)}:{line_idx}: Anchor '#{anchor}' not found in target '{target_path_str}'"
                        )

    return errors

def main():
    print("Linting repository documentation links...")
    md_files = find_markdown_files()
    headings_cache: Dict[Path, Set[str]] = {}
    all_errors = []
    total_files = len(md_files)

    for md_file in md_files:
        errors = lint_file(md_file, headings_cache)
        if errors:
            all_errors.extend(errors)

    if all_errors:
        print(f"\nFound {len(all_errors)} documentation link error(s):")
        for err in all_errors:
            print(f"  ❌ {err}")
        sys.exit(1)
    else:
        print(f"✓ Documentation link check passed across {total_files} Markdown files.")
        sys.exit(0)

if __name__ == "__main__":
    main()
