export interface BatchItem {
  path: string;
  filename: string;
  name: string;
  type: string;
  existingModId: string | null;
  existingModInfo?: any;
  existingVersion?: string | null;
  nexusModId?: number | null;
  version?: string | null;
  error?: string;
  hasPak?: boolean;
  isLogicModsDefault?: boolean;
}

export interface FileTreeNode {
  name: string;
  routeType?: string;
  destPath?: string;
  children: Map<string, FileTreeNode>;
}
