#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Marcus Baw
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Mine table/column/foreign-key facts out of an openMAXIMS T-SQL DDL dump.

The source file is not vendored in this repo (see reference/README.md for
why). Point this at your own checkout of the upstream openMAXIMS repo:

    python3 extract-ddl-schema.py \\
        "/path/to/openMAXIMS/Source Library/database/DDL-scripts-ref147-openMAXIMS.sql" \\
        ../legacy-schema

Writes legacy-schema.json (one record per table) and
legacy-schema-summary.md (per-module table/column counts) into the given
output directory.
"""

import json
import re
import sys
from collections import defaultdict
from pathlib import Path

CREATE_RE = re.compile(r"CREATE TABLE \[dbo\]\.\[(\w+)\]\(")
FK_RE = re.compile(
    r"ALTER TABLE \[dbo\]\.\[(\w+)\]\s+WITH CHECK ADD\s+CONSTRAINT \[(\w+)\] "
    r"FOREIGN KEY\(\[(\w+)\]\)\s*\n"
    r"REFERENCES \[dbo\]\.\[(\w+)\] \(\[(\w+)\]\)"
)
COLUMN_RE = re.compile(
    r"^\[(?P<name>\w+)\]\s+\[(?P<type>\w+)\](\((?P<params>[^)]*)\))?"
    r"(?P<identity>\s+IDENTITY\(\d+,\s*\d+\))?"
    r"\s+(?P<nullable>NOT NULL|NULL)"
)
PK_COLUMN_RE = re.compile(r"\[(\w+)\]\s*ASC")


def find_matching_paren(text: str, open_idx: int) -> int:
    depth = 1
    i = open_idx + 1
    while depth > 0:
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
        i += 1
    return i - 1


def split_top_level(body: str) -> list[str]:
    """Split a table body on commas that are not nested inside parens."""
    parts, current, depth = [], [], 0
    for c in body:
        if c == "(":
            depth += 1
            current.append(c)
        elif c == ")":
            depth -= 1
            current.append(c)
        elif c == "," and depth == 0:
            parts.append("".join(current))
            current = []
        else:
            current.append(c)
    if current:
        parts.append("".join(current))
    return [p.strip() for p in parts if p.strip()]


def parse_table(table_name: str, body: str) -> dict:
    columns, pk_columns = [], []
    for item in split_top_level(body):
        if item.upper().startswith("CONSTRAINT") and "PRIMARY KEY" in item.upper():
            pk_columns = PK_COLUMN_RE.findall(item)
            continue
        m = COLUMN_RE.match(item)
        if not m:
            continue
        columns.append(
            {
                "name": m.group("name"),
                "type": m.group("type"),
                "params": m.group("params"),
                "identity": bool(m.group("identity")),
                "nullable": m.group("nullable") == "NULL",
            }
        )
    return {
        "table": table_name,
        "module_prefix": table_name.split("_")[0],
        "columns": columns,
        "primary_key": pk_columns,
    }


def extract_tables(sql_text: str) -> list[dict]:
    tables = []
    for m in CREATE_RE.finditer(sql_text):
        open_idx = m.end() - 1
        close_idx = find_matching_paren(sql_text, open_idx)
        body = sql_text[open_idx + 1 : close_idx]
        tables.append(parse_table(m.group(1), body))
    return tables


def extract_foreign_keys(sql_text: str) -> list[dict]:
    fks = []
    for m in FK_RE.finditer(sql_text):
        table, constraint, column, ref_table, ref_column = m.groups()
        fks.append(
            {
                "table": table,
                "constraint": constraint,
                "column": column,
                "references_table": ref_table,
                "references_column": ref_column,
            }
        )
    return fks


def read_sql(path: Path) -> str:
    for encoding in ("utf-8-sig", "cp1252"):
        try:
            return path.read_text(encoding=encoding)
        except UnicodeDecodeError:
            continue
    return path.read_text(encoding="latin-1")


def main() -> None:
    if len(sys.argv) != 3:
        print(__doc__)
        sys.exit(1)

    src, out_dir = Path(sys.argv[1]), Path(sys.argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)

    sql_text = read_sql(src)
    tables = extract_tables(sql_text)

    fks_by_table = defaultdict(list)
    for fk in extract_foreign_keys(sql_text):
        fks_by_table[fk["table"]].append(fk)
    for table in tables:
        table["foreign_keys"] = fks_by_table.get(table["table"], [])

    (out_dir / "legacy-schema.json").write_text(json.dumps(tables, indent=2) + "\n")

    module_counts: dict[str, dict[str, int]] = defaultdict(lambda: {"tables": 0, "columns": 0, "fks": 0})
    for table in tables:
        counts = module_counts[table["module_prefix"]]
        counts["tables"] += 1
        counts["columns"] += len(table["columns"])
        counts["fks"] += len(table["foreign_keys"])

    total_columns = sum(len(t["columns"]) for t in tables)
    total_fks = sum(len(t["foreign_keys"]) for t in tables)
    lines = [
        "# Legacy schema summary (generated)",
        "",
        f"Mined from `{src.name}`: {len(tables)} tables, {total_columns} columns, {total_fks} foreign keys.",
        "",
        "Regenerate with `extract-ddl-schema.py`; do not hand-edit.",
        "",
        "| Module prefix | Tables | Columns | Foreign keys |",
        "|---|---|---|---|",
    ]
    for prefix, counts in sorted(module_counts.items(), key=lambda kv: -kv[1]["tables"]):
        lines.append(f"| `{prefix}_` | {counts['tables']} | {counts['columns']} | {counts['fks']} |")
    (out_dir / "legacy-schema-summary.md").write_text("\n".join(lines) + "\n")

    print(f"Parsed {len(tables)} tables ({total_columns} columns, {total_fks} FKs) -> {out_dir}")


if __name__ == "__main__":
    main()
