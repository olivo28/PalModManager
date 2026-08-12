import { addStagedPaths } from './staging';
import { stagedFiles, targetOverrides, renderWorkspace, virtualFolders, setVirtualFolders } from './mod';
import { showToast as toast } from '../toast';

function showToast(msg: string, type: 'success' | 'warning' | 'error' | 'info'): void {
  toast(msg, type);
}

export function setupPackerDragAndDrop(): void {
  const container = document.getElementById('build-view');
  const overlay = document.getElementById('packer-drag-overlay');
  if (!container || !overlay) return;

  ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
    container.addEventListener(eventName, (e: Event) => { e.preventDefault(); }, false);
  });

  container.addEventListener('dragenter', () => {
    if (!document.body.hasAttribute('data-internal-drag')) {
      overlay.classList.add('drag-over');
    }
  }, false);

  container.addEventListener('dragover', () => {
    if (!document.body.hasAttribute('data-internal-drag')) {
      overlay.classList.add('drag-over');
    }
  }, false);

  container.addEventListener('dragleave', () => {
    overlay.classList.remove('drag-over');
  }, false);
}

/** Moves all staged files and virtual folders from oldPath to newPath prefix */
function moveDirPath(oldPath: string, newPath: string): void {
  stagedFiles.forEach(f => {
    if (f.targetPath === oldPath) {
      f.targetPath = newPath;
      targetOverrides.set(f.sourcePath, newPath);
    } else if (f.targetPath.startsWith(oldPath + '/')) {
      f.targetPath = newPath + f.targetPath.substring(oldPath.length);
      targetOverrides.set(f.sourcePath, f.targetPath);
    }
  });

  const updatedVirtual = virtualFolders.map(vf => {
    if (vf === oldPath) return newPath;
    if (vf.startsWith(oldPath + '/')) return newPath + vf.substring(oldPath.length);
    return vf;
  });
  setVirtualFolders(updatedVirtual);

  // Clean up stale virtual dir overrides for old path
  targetOverrides.forEach((_val, key) => {
    if (key.startsWith('__VIRTUAL_DIR__:')) {
      const path = key.substring('__VIRTUAL_DIR__:'.length);
      if (path === oldPath || path.startsWith(oldPath + '/')) {
        targetOverrides.delete(key);
      }
    }
  });
  virtualFolders.forEach(vf => {
    targetOverrides.set(`__VIRTUAL_DIR__:${vf}`, '__VIRTUAL_DIR__');
  });
}

export function setupTreeDragAndDropHandlers(container: HTMLElement): void {
  // ── Pointer-event based DnD (HTML5 drag API is broken in Tauri/WebView2) ──
  let draggedIndex: number | null = null;
  let draggedDirPath: string | null = null;
  let ghost: HTMLElement | null = null;
  let activeDropTarget: HTMLElement | null = null;

  function clearDropHighlights(): void {
    container.querySelectorAll('.packer-tree-dir.drag-over').forEach(n => n.classList.remove('drag-over'));
    const root = container.querySelector('.packer-tree-root');
    if (root) root.classList.remove('drag-over');
  }

  function createGhost(label: string): HTMLElement {
    const g = document.createElement('div');
    g.style.cssText = `
      position: fixed; pointer-events: none; z-index: 9998;
      background: rgba(0,188,255,0.15); border: 1px solid var(--accent);
      color: var(--text-primary); font-size: 12px; padding: 4px 10px;
      border-radius: 6px; backdrop-filter: blur(6px); white-space: nowrap;
      box-shadow: 0 4px 16px rgba(0,0,0,0.4);
    `;
    g.textContent = `📦 ${label}`;
    document.body.appendChild(g);
    return g;
  }

  function getDropTarget(e: PointerEvent): { folderPath: string | null; isRoot: boolean } {
    // Check if pointer is over a dir node
    const dirNodes = Array.from(container.querySelectorAll('.packer-tree-dir')) as HTMLElement[];
    for (const node of dirNodes) {
      const rect = node.getBoundingClientRect();
      if (e.clientX >= rect.left && e.clientX <= rect.right &&
          e.clientY >= rect.top && e.clientY <= rect.bottom) {
        return { folderPath: node.dataset.path || '', isRoot: false };
      }
    }
    // Check root
    const root = container.querySelector('.packer-tree-root') as HTMLElement;
    if (root) {
      const rect = root.getBoundingClientRect();
      if (e.clientX >= rect.left && e.clientX <= rect.right &&
          e.clientY >= rect.top && e.clientY <= rect.bottom) {
        return { folderPath: null, isRoot: true };
      }
    }
    return { folderPath: null, isRoot: false };
  }

  // ── Attach pointerdown on each file node's icon/name (not action buttons) ──
  container.querySelectorAll('.packer-tree-file').forEach(node => {
    const fileNode = node as HTMLElement;
    fileNode.style.cursor = 'grab';

    fileNode.addEventListener('pointerdown', (e: PointerEvent) => {
      // Ignore right-clicks and other non-left clicks
      if (e.button !== 0) return;
      // Ignore clicks on action buttons
      if ((e.target as HTMLElement).closest('.packer-tree-actions')) return;
      e.preventDefault();

      draggedIndex = parseInt(fileNode.dataset.index || '0', 10);
      draggedDirPath = null;

      const filename = stagedFiles[draggedIndex]?.sourcePath.split(/[/\\]/).pop() || 'file';
      ghost = createGhost(filename);
      ghost.style.left = (e.clientX + 12) + 'px';
      ghost.style.top = (e.clientY + 8) + 'px';

      fileNode.style.opacity = '0.4';
      container.setPointerCapture(e.pointerId);
    });
  });

  // ── Attach pointerdown on each dir node ──
  container.querySelectorAll('.packer-tree-dir').forEach(node => {
    const dirNode = node as HTMLElement;
    dirNode.style.cursor = 'grab';

    dirNode.addEventListener('pointerdown', (e: PointerEvent) => {
      if (e.button !== 0) return;
      if ((e.target as HTMLElement).closest('.packer-tree-actions')) return;
      e.preventDefault();

      draggedDirPath = dirNode.dataset.path || '';
      draggedIndex = null;

      const dirname = draggedDirPath.split('/').pop() || draggedDirPath;
      ghost = createGhost('📁 ' + dirname);
      ghost.style.left = (e.clientX + 12) + 'px';
      ghost.style.top = (e.clientY + 8) + 'px';

      dirNode.style.opacity = '0.4';
      container.setPointerCapture(e.pointerId);
    });
  });

  // ── Shared pointermove on container (pointer is captured) ──
  container.addEventListener('pointermove', (e: PointerEvent) => {
    if (!ghost) return;
    e.preventDefault();

    ghost.style.left = (e.clientX + 12) + 'px';
    ghost.style.top = (e.clientY + 8) + 'px';

    // Highlight drop target
    clearDropHighlights();
    const { folderPath, isRoot } = getDropTarget(e);
    if (folderPath !== null) {
      const dirNodes = Array.from(container.querySelectorAll('.packer-tree-dir')) as HTMLElement[];
      const target = dirNodes.find(n => n.dataset.path === folderPath);
      if (target) { target.classList.add('drag-over'); activeDropTarget = target; }
    } else if (isRoot) {
      const root = container.querySelector('.packer-tree-root') as HTMLElement;
      if (root) { root.classList.add('drag-over'); activeDropTarget = root; }
    } else {
      activeDropTarget = null;
    }
  });

  // ── Shared pointerup ──
  container.addEventListener('pointerup', (e: PointerEvent) => {
    if (!ghost) return;
    e.preventDefault();

    ghost.remove();
    ghost = null;
    clearDropHighlights();
    activeDropTarget = null;

    // Restore opacity of all nodes
    container.querySelectorAll('.packer-tree-file, .packer-tree-dir').forEach(n => {
      (n as HTMLElement).style.opacity = '1';
    });

    const { folderPath, isRoot } = getDropTarget(e);

    if (draggedIndex !== null && draggedIndex >= 0 && draggedIndex < stagedFiles.length) {
      const file = stagedFiles[draggedIndex];
      const filename = file.sourcePath.split(/[/\\]/).pop() || file.relativePath;

      if (folderPath !== null) {
        // Dropped on a specific folder
        const newTarget = `${folderPath}/${filename}`;
        file.targetPath = newTarget.replace(/\\/g, '/');
        targetOverrides.set(file.sourcePath, file.targetPath);
        showToast(`Moved ${filename} → ${folderPath}`, 'success');
        renderWorkspace();
      } else if (isRoot) {
        // Dropped on root
        file.targetPath = filename;
        targetOverrides.set(file.sourcePath, filename);
        showToast(`Moved ${filename} → root`, 'success');
        renderWorkspace();
      }
    } else if (draggedDirPath !== null) {
      const oldPath = draggedDirPath;

      if (folderPath !== null) {
        // Dropped on another folder
        if (folderPath === oldPath || folderPath.startsWith(oldPath + '/')) {
          showToast('Cannot move folder inside itself', 'warning');
        } else {
          const dirname = oldPath.split('/').pop() || oldPath;
          const newPath = `${folderPath}/${dirname}`;
          moveDirPath(oldPath, newPath);
          showToast(`Moved 📁 ${dirname} → ${folderPath}`, 'success');
          renderWorkspace();
        }
      } else if (isRoot) {
        // Dropped on root
        const dirname = oldPath.split('/').pop() || oldPath;
        moveDirPath(oldPath, dirname);
        showToast(`Moved 📁 ${dirname} → root`, 'success');
        renderWorkspace();
      }
    }

    draggedIndex = null;
    draggedDirPath = null;
  });

  container.addEventListener('pointercancel', () => {
    if (ghost) { ghost.remove(); ghost = null; }
    clearDropHighlights();
    container.querySelectorAll('.packer-tree-file, .packer-tree-dir').forEach(n => {
      (n as HTMLElement).style.opacity = '1';
    });
    draggedIndex = null;
    draggedDirPath = null;
    activeDropTarget = null;
  });
}

export { addStagedPaths };
