import { useState, useCallback, useRef, useEffect, useMemo } from "react";
import { useApp } from "../context/AppContext";
import { useComments } from "../context/CommentsContext";
import type { FileStatus, ChangedFile } from "../types/git";
import { getDisplayStatus } from "../types/git";
import "./FileTree.css";

const STATUS_ICONS: Record<FileStatus, string> = {
  added: "+",
  modified: "~",
  deleted: "-",
  renamed: "→",
  copied: "©",
  untracked: "?",
};

const STATUS_CLASSES: Record<FileStatus, string> = {
  added: "status-added",
  modified: "status-modified",
  deleted: "status-deleted",
  renamed: "status-renamed",
  copied: "status-copied",
  untracked: "status-untracked",
};

interface TreeNode {
  name: string;
  path: string;
  isFolder: boolean;
  children: TreeNode[];
  file?: ChangedFile;
}

function buildTree(files: ChangedFile[]): TreeNode[] {
  const root: TreeNode[] = [];
  const folderMap = new Map<string, TreeNode>();

  for (const file of files) {
    const parts = file.path.split(/[/\\]/);
    let currentPath = "";
    let currentLevel = root;

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      currentPath = currentPath ? `${currentPath}/${part}` : part;

      if (isLast) {
        // This is a file
        currentLevel.push({
          name: part,
          path: file.path,
          isFolder: false,
          children: [],
          file,
        });
      } else {
        // This is a folder
        let folder = folderMap.get(currentPath);
        if (!folder) {
          folder = {
            name: part,
            path: currentPath,
            isFolder: true,
            children: [],
          };
          folderMap.set(currentPath, folder);
          currentLevel.push(folder);
        }
        currentLevel = folder.children;
      }
    }
  }

  // Sort: folders first, then files, both alphabetically
  const sortNodes = (nodes: TreeNode[]): TreeNode[] => {
    return nodes.sort((a, b) => {
      if (a.isFolder && !b.isFolder) return -1;
      if (!a.isFolder && b.isFolder) return 1;
      return a.name.localeCompare(b.name);
    });
  };

  const sortTree = (nodes: TreeNode[]): TreeNode[] => {
    const sorted = sortNodes(nodes);
    for (const node of sorted) {
      if (node.isFolder) {
        node.children = sortTree(node.children);
      }
    }
    return sorted;
  };

  return sortTree(root);
}

function flattenVisibleNodes(
  nodes: TreeNode[],
  expandedFolders: Set<string>,
  depth = 0,
): Array<{ node: TreeNode; depth: number }> {
  const result: Array<{ node: TreeNode; depth: number }> = [];

  for (const node of nodes) {
    result.push({ node, depth });
    if (node.isFolder && expandedFolders.has(node.path)) {
      result.push(
        ...flattenVisibleNodes(node.children, expandedFolders, depth + 1),
      );
    }
  }

  return result;
}

interface TreeItemProps {
  node: TreeNode;
  depth: number;
  isExpanded: boolean;
  isSelected: boolean;
  isFocused: boolean;
  onToggle: (path: string) => void;
  onSelect: (path: string) => void;
  onFocus: (path: string) => void;
  getCommentCount: (path: string) => number;
}

function TreeItem({
  node,
  depth,
  isExpanded,
  isSelected,
  isFocused,
  onToggle,
  onSelect,
  onFocus,
  getCommentCount,
}: TreeItemProps) {
  const itemRef = useRef<HTMLLIElement>(null);

  useEffect(() => {
    if (isFocused && itemRef.current) {
      itemRef.current.focus();
    }
  }, [isFocused]);

  const handleClick = () => {
    onFocus(node.path);
    if (node.isFolder) {
      onToggle(node.path);
    } else {
      onSelect(node.path);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleClick();
    }
  };

  const status = node.file ? getDisplayStatus(node.file) : null;
  const commentCount = node.file ? getCommentCount(node.file.path) : 0;

  return (
    <li
      ref={itemRef}
      className={`tree-item ${isSelected ? "selected" : ""} ${isFocused ? "focused" : ""}`}
      style={{ paddingLeft: `${12 + depth * 16}px` }}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      tabIndex={isFocused ? 0 : -1}
      role="treeitem"
      aria-selected={isSelected}
      aria-expanded={node.isFolder ? isExpanded : undefined}
      data-path={node.path}
    >
      {node.isFolder ? (
        <>
          <span className="folder-icon">{isExpanded ? "▼" : "▶"}</span>
          <span className="folder-name">{node.name}</span>
        </>
      ) : (
        <>
          <span
            className={`file-status ${status ? STATUS_CLASSES[status] : ""}`}
          >
            {status ? STATUS_ICONS[status] : " "}
          </span>
          <span className="file-name">{node.name}</span>
          {commentCount > 0 && (
            <span
              className="comment-badge"
              title={`${commentCount} comment${commentCount > 1 ? "s" : ""}`}
            >
              {commentCount}
            </span>
          )}
        </>
      )}
    </li>
  );
}

export function FileTree() {
  const { changedFiles, selectedFile, selectFile, refreshFiles } = useApp();
  const { getCommentCountForFile } = useComments();
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(
    new Set(),
  );
  const [focusedPath, setFocusedPath] = useState<string | null>(null);
  const treeRef = useRef<HTMLUListElement>(null);

  const tree = useMemo(() => buildTree(changedFiles), [changedFiles]);
  const visibleNodes = useMemo(
    () => flattenVisibleNodes(tree, expandedFolders),
    [tree, expandedFolders],
  );

  // Expand all folders by default when files change
  useEffect(() => {
    const allFolders = new Set<string>();
    const collectFolders = (nodes: TreeNode[]) => {
      for (const node of nodes) {
        if (node.isFolder) {
          allFolders.add(node.path);
          collectFolders(node.children);
        }
      }
    };
    collectFolders(tree);
    setExpandedFolders(allFolders);
  }, [tree]);

  const toggleFolder = useCallback((path: string) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const currentIndex = visibleNodes.findIndex(
        ({ node }) => node.path === focusedPath,
      );

      switch (e.key) {
        case "ArrowDown": {
          e.preventDefault();
          let targetNode: TreeNode | null = null;
          if (currentIndex < visibleNodes.length - 1) {
            targetNode = visibleNodes[currentIndex + 1].node;
          } else if (currentIndex === -1 && visibleNodes.length > 0) {
            targetNode = visibleNodes[0].node;
          }
          if (targetNode) {
            setFocusedPath(targetNode.path);
            if (!targetNode.isFolder) {
              selectFile(targetNode.path);
            }
          }
          break;
        }
        case "ArrowUp": {
          e.preventDefault();
          if (currentIndex > 0) {
            const targetNode = visibleNodes[currentIndex - 1].node;
            setFocusedPath(targetNode.path);
            if (!targetNode.isFolder) {
              selectFile(targetNode.path);
            }
          }
          break;
        }
        case "ArrowRight": {
          e.preventDefault();
          if (currentIndex >= 0) {
            const { node } = visibleNodes[currentIndex];
            if (node.isFolder) {
              if (!expandedFolders.has(node.path)) {
                toggleFolder(node.path);
              } else if (node.children.length > 0) {
                // Move to first child
                setFocusedPath(node.children[0].path);
              }
            }
          }
          break;
        }
        case "ArrowLeft": {
          e.preventDefault();
          if (currentIndex >= 0) {
            const { node, depth } = visibleNodes[currentIndex];
            if (node.isFolder && expandedFolders.has(node.path)) {
              // Collapse folder
              toggleFolder(node.path);
            } else if (depth > 0) {
              // Move to parent folder
              const parentPath = node.path.substring(
                0,
                node.path.lastIndexOf("/"),
              );
              if (parentPath) {
                setFocusedPath(parentPath);
              }
            }
          }
          break;
        }
        case "Enter": {
          e.preventDefault();
          if (currentIndex >= 0) {
            const { node } = visibleNodes[currentIndex];
            if (node.isFolder) {
              toggleFolder(node.path);
            } else {
              selectFile(node.path);
            }
          }
          break;
        }
      }
    },
    [focusedPath, visibleNodes, expandedFolders, toggleFolder, selectFile],
  );

  return (
    <div className="file-tree">
      <div className="file-tree-header">
        <h2>Changed Files</h2>
        <button
          className="refresh-btn"
          onClick={refreshFiles}
          title="Refresh files"
        >
          ↻
        </button>
      </div>
      <div className="file-tree-content">
        {changedFiles.length === 0 ? (
          <p className="empty-message">No changes detected</p>
        ) : (
          <ul
            ref={treeRef}
            className="file-tree-list"
            role="tree"
            aria-label="Changed files"
            onKeyDown={handleKeyDown}
          >
            {visibleNodes.map(({ node, depth }) => (
              <TreeItem
                key={node.path}
                node={node}
                depth={depth}
                isExpanded={expandedFolders.has(node.path)}
                isSelected={!node.isFolder && selectedFile === node.path}
                isFocused={focusedPath === node.path}
                onToggle={toggleFolder}
                onSelect={selectFile}
                onFocus={setFocusedPath}
                getCommentCount={getCommentCountForFile}
              />
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
