import type { BatchItem } from './types';

export let _pendingUpdateModId: string | null = null;
export let _pendingBatchPaths: string[] = [];
export let _batchItems: BatchItem[] = [];
export let _onInstallCompleteCallback: ((success: boolean) => void) | null = null;
export let _lastInstallSuccess = false;

export function setPendingUpdateModId(id: string | null): void {
  _pendingUpdateModId = id;
}

export function setPendingBatchPaths(paths: string[]): void {
  _pendingBatchPaths = paths;
}

export function setBatchItems(items: BatchItem[]): void {
  _batchItems = items;
}

export function setLastInstallSuccess(success: boolean): void {
  _lastInstallSuccess = success;
}

export function setInstallModalCallback(cb: ((success: boolean) => void) | null): void {
  _onInstallCompleteCallback = cb;
  _lastInstallSuccess = false;
}
