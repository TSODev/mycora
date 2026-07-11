---
id: 36c4e1db-5e0d-4cf6-b284-778dcc8103df
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 8
tags:
- features
- import
- v0.8
created: 2026-07-11T09:30:00Z
updated: 2026-07-11T09:30:00Z
---

# Importing an Obsidian vault

The other side of [[Roadmap]] v0.8's "notes are never trapped" goal —
[[Exporting a subtree]] gets content out, this gets an existing
Obsidian vault in.

```sh
mycora import <source> <name> <path>
```

CLI-only, no in-TUI equivalent — see
[[Folder structure becomes tree structure]] for the core design
tension this had to resolve (Obsidian has no `parent` field at all)
and why there's no `:import` command.

Per note, on the way in:

- **Title** comes from the filename, not a heading inside the file —
  Obsidian doesn't reliably have one the way a Mycora note always does.
- **Tags** carry over from frontmatter (single string or list form,
  Obsidian allows either). Every other frontmatter field is dropped;
  missing or unparseable frontmatter just means no tags, not an error.
- **Links**: an aliased or heading-anchored wikilink is rewritten down
  to a plain one, since [[Cross-links and backlinks]]'s resolution only
  understands bare double-square-bracket titles — without this, most
  real-world Obsidian links would come through silently broken.

Skips `.obsidian/` and anything that isn't a `.md` file. Always creates
a brand new vault and mounts it — refuses if the destination already
exists and isn't empty, same instinct as export's
refuse-on-existing-file.
