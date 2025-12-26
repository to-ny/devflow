Edit a cell in a Jupyter notebook.

Use when:
- Modifying notebook code or markdown
- Adding new cells to a notebook
- Removing cells from a notebook

Input:
- `path`: Relative path to .ipynb file
- `cell_number`: Zero-indexed cell position
- `new_source`: New cell content
- `cell_type`: "code" or "markdown" (optional, required for insert)
- `edit_mode`: "replace", "insert", or "delete" (optional, default "replace")

Modes:
- replace: Replace cell content at position
- insert: Add new cell after position
- delete: Remove cell at position

Errors if cell_number is out of range.
