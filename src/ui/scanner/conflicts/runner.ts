import { invoke } from '@tauri-apps/api/core';
import { showToast } from '../../toast';
import { t } from '../../../utils/i18n';
import {
  isScanning,
  setIsScanning,
  setLastScanResult,
  renderScannerView,
  type ScanResult,
} from '../mod';

export async function runScan(): Promise<void> {
  if (isScanning) return;
  setIsScanning(true);
  renderScannerView();

  try {
    const result = await invoke<ScanResult>('scan_conflicts');
    setLastScanResult(result);
    showToast(t('scanner.toast_scan_success', { count: result.totalScanned }), 'success');
  } catch (err: any) {
    console.error(err);
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  } finally {
    setIsScanning(false);
    renderScannerView();
  }
}
