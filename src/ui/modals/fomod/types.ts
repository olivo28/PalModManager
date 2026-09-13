export interface FomodInfo {
  name: string;
  author: string;
  version: string;
  description: string;
  website: string;
}

export interface FomodFileEntry {
  source: string;
  destination: string;
  priority: number;
  isFolder?: boolean;
}

export interface FomodFlag {
  name: string;
  value: string;
}

export interface FomodFlagDependency {
  flag: string;
  value: string;
}

export interface FomodVisibility {
  operator: 'And' | 'Or' | string;
  flagDependencies: FomodFlagDependency[];
}

export interface FomodPlugin {
  id: string;
  name: string;
  description: string;
  image?: string;
  imageBase64?: string;
  typeDescriptor: 'Required' | 'Recommended' | 'Optional' | 'NotUsable' | 'CouldBeUsable' | string;
  files: FomodFileEntry[];
  conditionFlags: FomodFlag[];
}

export type FomodGroupType =
  | 'SelectExactlyOne'
  | 'SelectAtLeastOne'
  | 'SelectAtMostOne'
  | 'SelectAny'
  | 'SelectAll'
  | string;

export interface FomodGroup {
  name: string;
  groupType: FomodGroupType;
  plugins: FomodPlugin[];
}

export interface FomodStep {
  name: string;
  visible?: FomodVisibility;
  groups: FomodGroup[];
}

export interface FomodPattern {
  dependencies: FomodVisibility;
  files: FomodFileEntry[];
}

export interface FomodConfig {
  moduleName: string;
  moduleImage?: string;
  bannerBase64?: string;
  info?: FomodInfo;
  requiredInstallFiles: FomodFileEntry[];
  installSteps: FomodStep[];
  conditionalFileInstalls: FomodPattern[];
}

export interface BuildFomodManifestPayload {
  zipPath: string;
  selectedFiles: FomodFileEntry[];
  customName?: string;
  customFolder?: string;
  modName?: string;
  author?: string;
  version?: string;
  summary?: string;
  fomodChoices?: Record<string, string[]>;
}
