import type { FomodConfig, FomodFileEntry, FomodPattern, FomodStep, FomodVisibility } from './types';

/**
 * Checks if a visibility/condition rule is satisfied by current flags.
 */
export function isVisibilitySatisfied(
  vis: FomodVisibility | undefined,
  flags: Map<string, string>
): boolean {
  if (!vis || !vis.flagDependencies || vis.flagDependencies.length === 0) {
    return true;
  }

  const isOr = (vis.operator || '').toLowerCase() === 'or';

  if (isOr) {
    return vis.flagDependencies.some((dep) => (flags.get(dep.flag) || '') === dep.value);
  }

  // Default: And
  return vis.flagDependencies.every((dep) => (flags.get(dep.flag) || '') === dep.value);
}

/**
 * Checks if a step is visible according to its visibility condition.
 */
export function isStepVisible(step: FomodStep, flags: Map<string, string>): boolean {
  return isVisibilitySatisfied(step.visible, flags);
}

/**
 * Recomputes active flags from all currently selected plugins across all steps.
 * When plugins are selected, their conditionFlags are registered.
 */
export function computeActiveFlags(
  config: FomodConfig,
  selections: Map<string, Set<string>>
): Map<string, string> {
  const flags = new Map<string, string>();

  for (const step of config.installSteps) {
    if (!isStepVisible(step, flags)) {
      continue;
    }

    for (const group of step.groups) {
      const selectedNames = selections.get(group.name);
      if (!selectedNames) continue;

      for (const plugin of group.plugins) {
        if (selectedNames.has(plugin.name)) {
          for (const flag of plugin.conditionFlags || []) {
            flags.set(flag.name, flag.value);
          }
        }
      }
    }
  }

  return flags;
}

/**
 * Evaluates conditional file patterns (<conditionalFileInstalls>) against active flags.
 */
export function evaluateConditionalPatterns(
  patterns: FomodPattern[],
  flags: Map<string, string>
): FomodFileEntry[] {
  const matchedFiles: FomodFileEntry[] = [];

  for (const pattern of patterns || []) {
    if (isVisibilitySatisfied(pattern.dependencies, flags)) {
      matchedFiles.push(...(pattern.files || []));
    }
  }

  return matchedFiles;
}
