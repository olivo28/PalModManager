import { t } from '../../../utils/i18n';

export function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export function formatDeathPenalty(penalty?: string | null): string {
  if (!penalty) return 'None';
  if (penalty.includes('None')) return t('scanner.death_none') || 'Keep All';
  if (penalty.includes('ItemAndEquipment')) return t('scanner.death_equipment') || 'Drop Items & Equipment';
  if (penalty.includes('Item')) return t('scanner.death_item') || 'Drop Items Only';
  if (penalty.includes('All')) return t('scanner.death_all') || 'Drop All (Items, Gear & Pals)';
  return penalty;
}
