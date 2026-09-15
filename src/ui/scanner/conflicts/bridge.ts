import { bus } from '../../../framework';
import type { ScanResult, UsmapHookDiagnostic } from '../mod';
import { lastScanResult, renderScannerView } from '../mod';
import type { EditorDiagnostic } from '../../../api';
import { injectScannerDiagnostics, markDiagnosticResolved } from '../../editor/monaco/problemsPanel';
import { isItemBroken } from './render/usmap_section';
import { t } from '../../../utils/i18n';

let _isBridgeInitialized = false;

/**
 * Initializes the reactive bridge between Monaco Editor and the Conflict Scanner.
 * Listens for conflict resolutions in the editor and updates the scanner view dynamically.
 */
export function initConflictResolverBridge(): void {
  if (_isBridgeInitialized) return;
  _isBridgeInitialized = true;

  bus.on('conflict:resolved', ({ modId, filePath, line, target }) => {
    if (!lastScanResult) return;
    let anyChanged = false;

    const normalizedPath = filePath.replace(/\\/g, '/').replace(/^\/+/, '');

    // 1. Check USMAP Diagnostics
    if (lastScanResult.usmapDiagnostics?.diagnostics) {
      for (const diag of lastScanResult.usmapDiagnostics.diagnostics) {
        const diagNorm = diag.filePath.replace(/\\/g, '/').replace(/^\/+/, '');
        if (diag.modId === modId && diagNorm === normalizedPath) {
          const matchLine = !line || diag.lineNumber === line;
          const matchTarget = !target || diag.hookTarget.toLowerCase().includes(target.toLowerCase());
          if (matchLine && matchTarget && !diag.resolved) {
            diag.resolved = true;
            anyChanged = true;
            if (lastScanResult.usmapDiagnostics.brokenHooks > 0) {
              lastScanResult.usmapDiagnostics.brokenHooks--;
            }
          }
        }
      }
    }

    // 2. Check Hook Conflicts
    if (lastScanResult.hookConflicts) {
      for (const hc of lastScanResult.hookConflicts) {
        for (const m of hc.mods) {
          const mNorm = m.filePath.replace(/\\/g, '/').replace(/^\/+/, '');
          if (m.modId === modId && mNorm === normalizedPath) {
            const matchLine = !line || m.lineNumber === line;
            if (matchLine && !m.resolved) {
              m.resolved = true;
              anyChanged = true;
            }
          }
        }
      }
    }

    // 3. Check Table Row Conflicts
    if (lastScanResult.tableConflicts) {
      for (const tc of lastScanResult.tableConflicts) {
        for (const m of tc.mods) {
          const mNorm = m.filePath.replace(/\\/g, '/').replace(/^\/+/, '');
          if (m.modId === modId && mNorm === normalizedPath) {
            const matchLine = !line || m.lineNumber === line;
            if (matchLine && !m.resolved) {
              m.resolved = true;
              anyChanged = true;
            }
          }
        }
      }
    }

    if (anyChanged) {
      // Also update Monaco problems panel
      if (line) {
        markDiagnosticResolved(modId, normalizedPath, line, target);
      }
      // Re-render scanner view if visible
      const scannerView = document.getElementById('scanner-view');
      if (scannerView && !scannerView.classList.contains('hidden')) {
        renderScannerView();
      }
    }
  });
}

/**
 * Transforms scan results into EditorDiagnostic structures and injects them into Monaco Editor.
 */
export function syncScanDiagnosticsToEditor(result: ScanResult): void {
  const diagnosticsByMod: Record<string, Record<string, EditorDiagnostic[]>> = {};

  function addDiag(modId: string, filePath: string, diag: EditorDiagnostic) {
    if (!diagnosticsByMod[modId]) {
      diagnosticsByMod[modId] = {};
    }
    const normalized = filePath.replace(/\\/g, '/').replace(/^\/+/, '');
    if (!diagnosticsByMod[modId][normalized]) {
      diagnosticsByMod[modId][normalized] = [];
    }
    diagnosticsByMod[modId][normalized].push(diag);
  }

  // 1. Process broken USMAP diagnostics
  if (result.usmapDiagnostics?.diagnostics) {
    for (const d of result.usmapDiagnostics.diagnostics) {
      if (!isItemBroken(d) || d.resolved) continue;

      const isError = d.status === 'broken_class' || d.status === 'broken_table' || d.status === 'broken_struct';
      addDiag(d.modId, d.filePath, {
        line: d.lineNumber,
        column: 1,
        severity: isError ? 'error' : 'warning',
        message: d.reason,
        target: d.hookTarget,
        suggestion: d.suggestion,
        category: d.category || 'ue4ss',
      });
    }
  }

  // 2. Process Multi-mod Hook Conflicts
  if (result.hookConflicts) {
    for (const hc of result.hookConflicts) {
      for (const m of hc.mods) {
        if (m.resolved) continue;
        const otherMods = hc.mods
          .filter(other => other.modId !== m.modId)
          .map(other => other.modName)
          .join(', ');

        const collisionPrefix = t('editor.problems_collision_with', { mod: otherMods || 'Other Mod' }) || `Collides with "${otherMods}"`;
        addDiag(m.modId, m.filePath, {
          line: m.lineNumber,
          column: 1,
          severity: 'warning',
          message: `${collisionPrefix}: ${hc.hookTarget}`,
          target: hc.hookTarget,
          category: 'conflict',
        });
      }
    }
  }

  // 3. Process Table Row Conflicts
  if (result.tableConflicts) {
    for (const tc of result.tableConflicts) {
      for (const m of tc.mods) {
        if (m.resolved) continue;
        const otherMods = tc.mods
          .filter(other => other.modId !== m.modId)
          .map(other => other.modName)
          .join(', ');

        const collisionPrefix = t('editor.problems_collision_with', { mod: otherMods || 'Other Mod' }) || `Collides with "${otherMods}"`;
        addDiag(m.modId, m.filePath, {
          line: m.lineNumber,
          column: 1,
          severity: 'warning',
          message: `${collisionPrefix}: ${tc.tableName} -> ${tc.rowName}`,
          target: tc.rowName,
          category: 'conflict',
        });
      }
    }
  }

  injectScannerDiagnostics(diagnosticsByMod);
}
