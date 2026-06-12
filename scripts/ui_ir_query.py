#!/usr/bin/env python3
"""Query composed UI IR JSON (plan P1.1, ledger items 19/23).

Input: an IR document produced by `ui render --dump-ir-dir <dir>` (one
`*.ir.json` per helper; see docs/ui-reference.md §6).

Subcommands:
  query <ir.json> <regex> [--fields a.b,c]
      One line per node whose `name` OR `text_payload.text` matches the
      regex (re.search, case-sensitive). Always prints id, parent,
      node_type, name, computed_rect, is_active; `--fields` appends extra
      dotted-path lookups into the node JSON (e.g.
      `text_payload.text,text_style.font_size`).
  tree <ir.json> <node_id>
      Ancestor chain (root first, indented) for one node: computed_rect,
      authored_size, anchor/pivot, padding, margin.

Dependency-free: stdlib json/re/argparse only.
"""

import argparse
import json
import re
import sys


def load_nodes(path):
    with open(path) as handle:
        doc = json.load(handle)
    nodes = doc.get("nodes")
    if not isinstance(nodes, list):
        sys.exit(f"error: {path} has no 'nodes' array — is it an IR JSON from --dump-ir-dir?")
    return doc, {node["id"]: node for node in nodes}, nodes


def fmt_rect(rect):
    if not isinstance(rect, dict):
        return "none"
    return "({x:g},{y:g},{w:g},{h:g})".format(
        x=rect.get("x", 0.0), y=rect.get("y", 0.0),
        w=rect.get("w", 0.0), h=rect.get("h", 0.0),
    )


def lookup_path(node, dotted):
    value = node
    for part in dotted.split("."):
        if isinstance(value, dict):
            value = value.get(part)
        elif isinstance(value, list) and part.isdigit():
            index = int(part)
            value = value[index] if index < len(value) else None
        else:
            return None
        if value is None:
            return None
    return value


def cmd_query(args):
    _, _, nodes = load_nodes(args.ir_json)
    pattern = re.compile(args.regex)
    fields = [field for field in (args.fields or "").split(",") if field]
    matched = 0
    for node in nodes:
        name = node.get("name") or ""
        text = ((node.get("text_payload") or {}).get("text")) or ""
        if not (pattern.search(name) or pattern.search(text)):
            continue
        matched += 1
        row = (
            f"id={node.get('id')} parent={node.get('parent_id')} "
            f"type={node.get('node_type')} active={node.get('is_active')} "
            f"rect={fmt_rect(node.get('computed_rect'))} name={name!r}"
        )
        for field in fields:
            row += f" {field}={json.dumps(lookup_path(node, field))}"
        print(row)
    if matched == 0:
        print(f"no nodes matched {args.regex!r} (searched name + text_payload.text)",
              file=sys.stderr)
        return 1
    return 0


def cmd_tree(args):
    _, by_id, _ = load_nodes(args.ir_json)
    node = by_id.get(args.node_id)
    if node is None:
        sys.exit(f"error: node id {args.node_id} not in {args.ir_json}")
    chain = [node]
    seen = {args.node_id}
    while chain[0].get("parent_id") is not None:
        parent_id = chain[0]["parent_id"]
        if parent_id in seen or parent_id not in by_id:
            break  # cycle or dangling parent: stop rather than loop
        seen.add(parent_id)
        chain.insert(0, by_id[parent_id])
    for depth, entry in enumerate(chain):
        print(
            "{indent}id={id} type={ty} name={name!r} rect={rect} "
            "authored_size={size} anchor={anchor} pivot={pivot} "
            "padding={padding} margin={margin}".format(
                indent="  " * depth,
                id=entry.get("id"),
                ty=entry.get("node_type"),
                name=entry.get("name") or "",
                rect=fmt_rect(entry.get("computed_rect")),
                size=json.dumps(entry.get("authored_size")),
                anchor=json.dumps(entry.get("anchor")),
                pivot=json.dumps(entry.get("pivot")),
                padding=json.dumps(entry.get("padding")),
                margin=json.dumps(entry.get("margin")),
            )
        )
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)

    query = sub.add_parser("query", help="list nodes matching a regex on name or text")
    query.add_argument("ir_json")
    query.add_argument("regex")
    query.add_argument("--fields", help="comma-separated dotted paths to also print")
    query.set_defaults(func=cmd_query)

    tree = sub.add_parser("tree", help="ancestor chain with layout fields for one node")
    tree.add_argument("ir_json")
    tree.add_argument("node_id", type=int)
    tree.set_defaults(func=cmd_tree)

    args = parser.parse_args()
    sys.exit(args.func(args))


if __name__ == "__main__":
    main()
