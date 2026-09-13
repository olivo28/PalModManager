import { getFomodConfig } from '../../../api';
import { initFomodState } from './state';
import { closeFomodModal, openFomodModal } from './wizard';
import { initFomodListeners } from './listeners';

export * from './types';
export * from './state';
export * from './flags';
export * from './wizard';
export * from './listeners';
export * from './version';

const fomodConfigCache = new Map<string, Promise<any>>();

export function preloadFomodConfig(zipPath: string): void {
  if (!zipPath || fomodConfigCache.has(zipPath)) return;
  const promise = getFomodConfig(zipPath).catch((err) => {
    fomodConfigCache.delete(zipPath);
    throw err;
  });
  fomodConfigCache.set(zipPath, promise);
}

export async function openFomodWizard(
  zipPath: string,
  existingMod: { id: string; name: string; version: string; fomodChoices?: Record<string, string[]> | null } | null = null,
  options?: { customName?: string; version?: string } | string
): Promise<void> {
  try {
    initFomodListeners();

    let configPromise = fomodConfigCache.get(zipPath);
    if (!configPromise) {
      configPromise = getFomodConfig(zipPath);
    }
    const config = await configPromise;
    console.log('[FOMOD] Config loaded:', config.moduleName, 'Steps:', config.installSteps?.length);

    const parsedOptions = typeof options === 'string' ? { customName: options } : options;
    initFomodState(config, zipPath, existingMod, parsedOptions);

    openFomodModal();
  } catch (err) {
    console.error('[FOMOD] Failed to open FOMOD wizard:', err);
    const { bus } = await import('../../../framework');
    bus.emit('toast:show', {
      message: `FOMOD Error: ${String(err)}`,
      type: 'error',
    });
  }
}

export function closeFomodWizard(): void {
  closeFomodModal();
}
