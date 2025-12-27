Edit a cell in a Jupyter notebook (.ipynb file).

## Parameters

- `path`: Relative path to .ipynb file (required)
- `cell_number`: Zero-indexed cell position (required)
- `new_source`: New cell content (required)
- `cell_type`: "code" or "markdown" (optional, required for insert mode)
- `edit_mode`: "replace", "insert", or "delete" (optional, default "replace")

## Edit Modes

| Mode | Description |
|------|-------------|
| `replace` | Replace cell content at the specified position |
| `insert` | Add a new cell after the specified position |
| `delete` | Remove the cell at the specified position |

## When to Use

- Modifying notebook code or markdown cells
- Adding new cells to a notebook
- Removing cells from a notebook
- Updating analysis code

## Important Notes

- Cell numbers are zero-indexed
- Will error if cell_number is out of range
- For insert mode, cell_type is required
- For delete mode, new_source content is ignored

## Examples

Replace cell content:
```json
{
  "path": "notebooks/analysis.ipynb",
  "cell_number": 2,
  "new_source": "import pandas as pd\nimport numpy as np"
}
```

Insert new cell:
```json
{
  "path": "notebooks/analysis.ipynb",
  "cell_number": 3,
  "new_source": "# Data Visualization",
  "cell_type": "markdown",
  "edit_mode": "insert"
}
```

Delete cell:
```json
{
  "path": "notebooks/analysis.ipynb",
  "cell_number": 5,
  "new_source": "",
  "edit_mode": "delete"
}
```
