Read a Jupyter notebook (.ipynb) file including cell contents and outputs.

## Usage

- Use this instead of read_file for .ipynb files
- Returns all cells with their content and outputs

## Parameters

- `path`: Relative path to .ipynb file (required)

## Return Format

Returns all cells with:
- Cell number (zero-indexed)
- Cell type (code or markdown)
- Source content
- Outputs (for executed code cells)

## When to Use

- Examining notebook structure and code
- Understanding data analysis workflows
- Reviewing notebook outputs and visualizations
- Checking cell execution order

## When NOT to Use

- Regular text/code files (use read_file instead)
- Editing notebooks (use notebook_edit instead)

## Example

```json
{"path": "notebooks/analysis.ipynb"}
```
