import type { FomodConfig, FomodFileEntry, FomodGroup, FomodStep } from './types';
import { computeActiveFlags, evaluateConditionalPatterns, isStepVisible } from './flags';
import { getState } from '../../../state';

export interface FomodState {
  config: FomodConfig | null;
  zipPath: string;
  customName?: string;
  version?: string;
  existingMod?: any;
  currentStepIndex: number;
  openAccordionGroup: string | null;
  selections: Map<string, Set<string>>; // group.name -> Set of plugin.name
  flags: Map<string, string>;
  rememberedSelections: Set<string>; // "${group.name}::${plugin.name}"
}

const state: FomodState = {
  config: null,
  zipPath: '',
  currentStepIndex: 0,
  openAccordionGroup: null,
  selections: new Map(),
  flags: new Map(),
  rememberedSelections: new Set(),
};

export function getFomodState(): FomodState {
  return state;
}

export function initFomodState(
  config: FomodConfig,
  zipPath: string,
  existingMod?: any,
  options?: { customName?: string; version?: string }
): void {
  state.config = config;
  state.zipPath = zipPath;
  state.existingMod = existingMod;
  state.customName = options?.customName;
  state.version = options?.version;
  state.currentStepIndex = 0;
  state.openAccordionGroup = null;
  state.selections = new Map();
  state.flags = new Map();
  state.rememberedSelections = new Set();

  let savedChoices: Record<string, string[]> | undefined | null = existingMod?.fomodChoices;
  if (!savedChoices && existingMod) {
    try {
      const allMods = getState()?.allMods;
      const found = allMods?.find(
        (m) => (existingMod.id && m.id === existingMod.id) || (existingMod.name && m.name === existingMod.name)
      );
      if (found?.fomodChoices) {
        savedChoices = found.fomodChoices;
      }
    } catch {
      // ignore
    }
  }

  // Pre-populate selections for each group based on saved choices or defaults
  for (const step of config.installSteps || []) {
    for (const group of step.groups || []) {
      const selectedSet = new Set<string>();
      const savedGroupPlugins = savedChoices ? savedChoices[group.name] : undefined;

      if (savedGroupPlugins && savedGroupPlugins.length > 0) {
        let validSaved = savedGroupPlugins.filter((savedName) =>
          group.plugins.some((p) => p.name === savedName && p.typeDescriptor !== 'NotUsable')
        );

        if (group.groupType === 'SelectExactlyOne' || group.groupType === 'SelectAtMostOne') {
          if (validSaved.length > 1) {
            validSaved = [validSaved[0]];
          }
        }

        if (validSaved.length > 0) {
          for (const pluginName of validSaved) {
            selectedSet.add(pluginName);
            state.rememberedSelections.add(`${group.name}::${pluginName}`);
          }
        }
      }

      // If no valid saved selections were found or applied for this group, fallback to defaults
      if (selectedSet.size === 0) {
        if (group.groupType === 'SelectAll') {
          for (const p of group.plugins) {
            if (p.typeDescriptor !== 'NotUsable') {
              selectedSet.add(p.name);
            }
          }
        } else if (group.groupType === 'SelectExactlyOne') {
          const preferred =
            group.plugins.find((p) => p.typeDescriptor === 'Required') ||
            group.plugins.find((p) => p.typeDescriptor === 'Recommended') ||
            group.plugins.find((p) => p.name.toLowerCase().includes('(recommended)')) ||
            group.plugins.find((p) => p.typeDescriptor !== 'NotUsable');

          if (preferred) {
            selectedSet.add(preferred.name);
          }
        } else {
          // SelectAny, SelectAtLeastOne, SelectAtMostOne
          for (const p of group.plugins) {
            if (
              p.typeDescriptor === 'Required' ||
              p.typeDescriptor === 'Recommended' ||
              p.name.toLowerCase().includes('(recommended)')
            ) {
              selectedSet.add(p.name);
            }
          }
        }
      }

      state.selections.set(group.name, selectedSet);
    }
  }

  // Initial flag evaluation
  state.flags = computeActiveFlags(config, state.selections);
}

export function getVisibleSteps(): FomodStep[] {
  if (!state.config) return [];
  return (state.config.installSteps || []).filter((step) => isStepVisible(step, state.flags));
}

export function getCurrentVisibleStep(): FomodStep | null {
  const visible = getVisibleSteps();
  if (state.currentStepIndex < 0 || state.currentStepIndex >= visible.length) {
    return null;
  }
  return visible[state.currentStepIndex];
}

export function setPluginSelection(
  group: FomodGroup,
  pluginName: string,
  isSelected: boolean
): void {
  let groupSet = state.selections.get(group.name);
  if (!groupSet) {
    groupSet = new Set();
    state.selections.set(group.name, groupSet);
  }

  if (group.groupType === 'SelectExactlyOne') {
    groupSet.clear();
    if (isSelected) {
      groupSet.add(pluginName);
    }
  } else if (group.groupType === 'SelectAtMostOne') {
    if (isSelected) {
      groupSet.clear();
      groupSet.add(pluginName);
    } else {
      groupSet.delete(pluginName);
    }
  } else {
    // SelectAny, SelectAtLeastOne, SelectAll
    if (isSelected) {
      groupSet.add(pluginName);
    } else {
      groupSet.delete(pluginName);
    }
  }

  if (state.config) {
    state.flags = computeActiveFlags(state.config, state.selections);
  }
}

export function isGroupValid(group: FomodGroup): boolean {
  const groupSet = state.selections.get(group.name);
  const count = groupSet ? groupSet.size : 0;

  switch (group.groupType) {
    case 'SelectExactlyOne':
      return count === 1;
    case 'SelectAtLeastOne':
      return count >= 1;
    case 'SelectAtMostOne':
      return count <= 1;
    case 'SelectAll':
      return count === group.plugins.filter((p) => p.typeDescriptor !== 'NotUsable').length;
    case 'SelectAny':
    default:
      return true;
  }
}

export function isStepValid(step: FomodStep | null | undefined): boolean {
  if (!step) return true;
  for (const group of step.groups || []) {
    if (!isGroupValid(group)) {
      return false;
    }
  }
  return true;
}

export function isCurrentStepValid(): boolean {
  return isStepValid(getCurrentVisibleStep());
}

export function collectAllSelectedFiles(): FomodFileEntry[] {
  if (!state.config) return [];

  const files: FomodFileEntry[] = [];

  // 1. Module required files
  if (state.config.requiredInstallFiles) {
    files.push(...state.config.requiredInstallFiles);
  }

  // 2. Visible steps selected plugins files
  const visibleSteps = getVisibleSteps();
  for (const step of visibleSteps) {
    for (const group of step.groups) {
      const selectedNames = state.selections.get(group.name);
      if (!selectedNames) continue;

      for (const plugin of group.plugins) {
        if (selectedNames.has(plugin.name) && plugin.files) {
          files.push(...plugin.files);
        }
      }
    }
  }

  // 3. Conditional pattern files
  if (state.config.conditionalFileInstalls) {
    const conditionalFiles = evaluateConditionalPatterns(
      state.config.conditionalFileInstalls,
      state.flags
    );
    files.push(...conditionalFiles);
  }

  return files;
}
