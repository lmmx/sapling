# ¶ ⠶ sapling

[![PyPI](https://img.shields.io/pypi/v/sapling?color=%2300dc00)](https://pypi.org/project/sapling)
[![crates.io](https://img.shields.io/crates/v/sapling.svg)](https://crates.io/crates/sapling)
[![documentation](https://docs.rs/sapling/badge.svg)](https://docs.rs/sapling)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/sapling.svg)](./LICENSE)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/lmmx/sapling/master.svg)](https://results.pre-commit.ci/latest/github/lmmx/sapling/master)

A syntactic patching library with char-level granularity.

## Installation

```bash
pip install sapling
```

## Quick Start

```python
import sapling

# Create a simple patch
patch = sapling.Patch.from_literal_target(
    file="example.txt",
    needle="old text",
    mode="include",
    replacement="new text"
)

# Apply to string content
content = "This is old text in a file"
result = patch.apply_to_string(content)
print(result)  # "This is new text in a file"

# Work with multiple patches
patchset = sapling.PatchSet()
patchset.add(patch)

# Apply to actual files
results = patchset.apply_to_files()
```

## Advanced Usage

### Using Snippets and Boundaries

```python
# Create a target
target = sapling.Target.literal("hello")

# Create a boundary with mode
boundary = sapling.Boundary(target, "include")

# Create a snippet
snippet = sapling.Snippet.at(boundary)

# Create a patch with the snippet
patch = sapling.Patch(
    file="test.txt",
    snippet=snippet,
    replacement="goodbye"
)
```

### Line-based Patching

```python
# Delete lines 5-10
patch = sapling.Patch.from_line_range(
    file="large_file.txt",
    start_line=5,
    end_line=10,
    replacement=""
)
```

### Between Markers

```python
# Replace content between HTML comments
start = sapling.Boundary(
    sapling.Target.literal("<!-- start -->"),
    "exclude"
)
end = sapling.Boundary(
    sapling.Target.literal("<!-- end -->"),
    "exclude"
)

snippet = sapling.Snippet.between(start, end)

patch = sapling.Patch(
    file="template.html",
    snippet=snippet,
    replacement="new content"
)
```

### JSON Import/Export

```python
# Load patches from JSON
json_data = '[{"file": "test.txt", ...}]'
patches = sapling.load_patches_from_json(json_data)

# Save patches to JSON
json_str = sapling.save_patches_to_json(patches)
```

## Licensing

Sapling is [MIT licensed](https://github.com/lmmx/sapling/blob/master/LICENSE), a permissive open source license.
