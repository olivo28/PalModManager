import * as monaco from 'monaco-editor';
import { getPalSchemaMonacoDefinitions, getPalSchemaRawSchema, PalSchemaDefinition } from '../../../api';

let cachedSchemaDefinitions: PalSchemaDefinition[] = [];
let _isInitialized = false;
let _fullSchemasLoaded = false;
const registeredRawTables = new Set<string>();
const parsedSchemaMap = new Map<string, any>();

/**
 * Initializes and registers official PalSchema JSON schemas into Monaco's JSON language service.
 * - Stage 1 (0ms): Loads essential domain schemas so editor renders instantly.
 * - Stage 2 (100ms in background): Loads all 474 raw DataTables schemas for full Okaetsu RawTable autocompletion.
 */
export async function initializeMonacoPalSchemas(force = false): Promise<void> {
  if (_isInitialized && !force) return;
  _isInitialized = true;

  try {
    // Stage 1: Fast core domain schemas (<1ms)
    const coreDefs = await getPalSchemaMonacoDefinitions(false);
    if (coreDefs && coreDefs.length > 0) {
      cachedSchemaDefinitions = coreDefs;
      applySchemasToMonaco(coreDefs);
      console.info(`PalSchema Schemas: Initialized ${coreDefs.length} core domain schemas.`);
    }

    // Stage 2: Background full raw schemas enrichment (non-blocking)
    scheduleFullSchemasLoading(force);
  } catch (err) {
    console.error('Failed to initialize Monaco PalSchema schemas:', err);
  }
}

/**
 * Loads all 481 PalSchema definitions (including all 474 DataTables) in the background without UI blocking.
 */
function scheduleFullSchemasLoading(force = false): void {
  if (_fullSchemasLoaded && !force) return;

  setTimeout(async () => {
    try {
      const fullDefs = await getPalSchemaMonacoDefinitions(true);
      if (fullDefs && fullDefs.length > 0) {
        cachedSchemaDefinitions = fullDefs;
        applySchemasToMonaco(fullDefs);
        _fullSchemasLoaded = true;
        console.info(`PalSchema Schemas: Enriched with all ${fullDefs.length} schemas (474 DataTables active).`);
      }
    } catch (err) {
      console.warn('Failed to background-load full PalSchema definitions:', err);
    }
  }, 100);
}

/**
 * Registers an individual raw DataTable schema on-demand when a user opens a DT_*.json file.
 */
export async function registerRawTableSchema(tableName: string): Promise<void> {
  const cleanName = tableName.trim().replace(/\.jsonc?$/i, '').replace(/\.schema$/i, '');
  if (!cleanName || registeredRawTables.has(cleanName.toLowerCase())) return;

  registeredRawTables.add(cleanName.toLowerCase());
  try {
    const def = await getPalSchemaRawSchema(cleanName);
    if (def) {
      cachedSchemaDefinitions.push(def);
      applySchemasToMonaco(cachedSchemaDefinitions);
      console.info(`PalSchema Schemas: Dynamically registered schema for table ${cleanName}`);
    }
  } catch (err) {
    console.warn(`Failed to dynamically register schema for ${cleanName}:`, err);
  }
}

/**
 * Applies schema definitions to Monaco's jsonDefaults diagnostics options.
 */
function applySchemasToMonaco(defs: PalSchemaDefinition[]): void {
  const jsonLang = (monaco.languages as any).json;
  if (!jsonLang || !jsonLang.jsonDefaults) return;

  const monacoSchemas = defs.map((def) => {
    let parsedSchema = parsedSchemaMap.get(def.uri);
    if (!parsedSchema) {
      try {
        parsedSchema = JSON.parse(def.schema_json);
        if (parsedSchema && typeof parsedSchema === 'object') {
          parsedSchema.allowComments = true;
          parsedSchema.allowTrailingCommas = true;
        }
        parsedSchemaMap.set(def.uri, parsedSchema);
      } catch (e) {
        console.warn(`Failed to parse JSON for schema ${def.uri}:`, e);
        parsedSchema = { allowComments: true, allowTrailingCommas: true };
      }
    } else {
      parsedSchema.allowComments = true;
      parsedSchema.allowTrailingCommas = true;
    }

    const sanitizedMatches = def.file_match && def.file_match.length > 0
      ? def.file_match.map((pattern) => {
          if (!pattern.startsWith('**') && !pattern.startsWith('file:') && !pattern.startsWith('http')) {
            return pattern.startsWith('*') ? `**/${pattern}` : `**/*${pattern}*`;
          }
          return pattern;
        })
      : undefined;

    return {
      uri: def.uri,
      fileMatch: sanitizedMatches,
      schema: parsedSchema,
    };
  });

  jsonLang.jsonDefaults.setDiagnosticsOptions({
    validate: true,
    allowComments: true,
    comments: 'ignore',
    trailingCommas: 'ignore',
    schemaValidation: 'warning',
    enableSchemaRequest: false,
    schemas: monacoSchemas,
  });

  if (typeof jsonLang.jsonDefaults.setModeConfiguration === 'function') {
    jsonLang.jsonDefaults.setModeConfiguration({
      documentFormattingEdits: true,
      documentRangeFormattingEdits: true,
      completionItems: true, // Enable schema-driven property & enum autocomplete
      hovers: true,
      documentSymbols: true,
      tokens: true,
      colors: true,
      foldingRanges: true,
      selectionRanges: true,
    });
  }
}

/**
 * Explicitly binds an opened model URI directly to its matching PalSchema definition in Monaco.
 */
export function associateFileWithPalSchema(filePath: string): void {
  const jsonLang = (monaco.languages as any).json;
  if (!jsonLang || !jsonLang.jsonDefaults) return;

  const modelUri = monaco.Uri.file(filePath).toString();
  const lower = filePath.toLowerCase().replace(/\\/g, '/');

  let targetSchemaUri: string | null = null;
  if (lower.includes('/items') || lower.endsWith('items.json') || lower.endsWith('items.jsonc') || lower.endsWith('item.json') || lower.endsWith('item.jsonc')) {
    targetSchemaUri = 'palschema://schemas/items.schema.json';
  } else if (lower.includes('/pals') || lower.endsWith('pals.json') || lower.endsWith('pals.jsonc') || lower.endsWith('pal.json') || lower.endsWith('pal.jsonc')) {
    targetSchemaUri = 'palschema://schemas/pals.schema.json';
  } else if (lower.includes('/buildings') || lower.endsWith('buildings.json') || lower.endsWith('buildings.jsonc')) {
    targetSchemaUri = 'palschema://schemas/buildings.schema.json';
  } else if (lower.includes('/skins') || lower.endsWith('skins.json') || lower.endsWith('skins.jsonc')) {
    targetSchemaUri = 'palschema://schemas/skins.schema.json';
  } else if (lower.includes('/utility') || lower.endsWith('utility.json') || lower.endsWith('utility.jsonc')) {
    targetSchemaUri = 'palschema://schemas/utility.schema.json';
  } else if (lower.includes('dt_') || lower.includes('/raw/')) {
    targetSchemaUri = 'palschema://schemas/raw.schema.json';
  }

  if (!targetSchemaUri) return;

  const currentOpts = jsonLang.jsonDefaults.diagnosticsOptions;
  if (!currentOpts || !currentOpts.schemas) return;

  let needsUpdate = false;
  for (const s of currentOpts.schemas) {
    if (s.uri === targetSchemaUri) {
      if (!s.fileMatch) {
        s.fileMatch = [modelUri];
        needsUpdate = true;
      } else if (!s.fileMatch.includes(modelUri)) {
        s.fileMatch.push(modelUri);
        needsUpdate = true;
      }
      break;
    }
  }

  if (needsUpdate) {
    jsonLang.jsonDefaults.setDiagnosticsOptions({
      ...currentOpts,
      allowComments: true,
      comments: 'ignore',
      trailingCommas: 'ignore',
      schemas: [...currentOpts.schemas],
    });
    console.info(`PalSchema: Explicitly associated model ${modelUri} with schema ${targetSchemaUri}`);
  }
}

/**
 * Reloads and refreshes the schemas in Monaco (e.g. after a catalog sync).
 */
export async function refreshMonacoPalSchemas(): Promise<void> {
  await initializeMonacoPalSchemas(true);
}

/**
 * Returns cached definitions for external inspector tools or linters.
 */
export function getCachedPalSchemaDefinitions(): PalSchemaDefinition[] {
  return cachedSchemaDefinitions;
}

