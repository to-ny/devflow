Read a Jupyter notebook file including cell contents and outputs.

Use when:
- Examining notebook structure and code
- Understanding data analysis workflows
- Reviewing notebook outputs

Input:
- `path`: Relative path to .ipynb file

Returns all cells with:
- Cell number and type (code/markdown)
- Source content
- Outputs (for executed code cells)

For regular files, use read_file instead.
