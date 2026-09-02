/**
 * PalModManager - DataTable Symbol Index Generator
 * 
 * Scans cooked Palworld JSON DataTable dumps, extracts table names, package paths,
 * RowStruct mappings, and row keys, generating a compact symbol index for Monaco IntelliSense.
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const DEFAULT_SOURCE_DIR = 'C:\\Users\\Antikux\\Documents\\PalWorld Mods\\Palworld Mod Maker\\SDK & References\\DataTables';
const TARGET_DIR = path.resolve(__dirname, '..', 'resources', 'datatables');

const sourceDir = process.argv[2] || DEFAULT_SOURCE_DIR;

console.log('=== Palworld DataTables Index Generator ===');
console.log(`Source Directory: ${sourceDir}`);
console.log(`Target Directory: ${TARGET_DIR}`);

if (!fs.existsSync(sourceDir)) {
  console.error(`Error: Source directory does not exist: ${sourceDir}`);
  process.exit(1);
}

function walkDirectory(dir) {
  let files = [];
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkDirectory(fullPath));
    } else if (entry.isFile() && entry.name.endsWith('.json')) {
      files.push(fullPath);
    }
  }
  return files;
}

console.log('Scanning JSON files...');
const jsonFiles = walkDirectory(sourceDir);
console.log(`Found ${jsonFiles.length} JSON files.`);

const tables = {};
let totalRows = 0;

for (const file of jsonFiles) {
  try {
    const raw = fs.readFileSync(file, 'utf8');
    const parsed = JSON.parse(raw);
    const dt = Array.isArray(parsed) ? parsed.find(x => x && x.Type === 'DataTable') : null;
    if (!dt || !dt.Name) continue;

    const tableName = dt.Name;
    const rawRowStruct = dt.Properties?.RowStruct?.ObjectName || '';
    const cleanStruct = rawRowStruct
      .replace(/^ScriptStruct'/, '')
      .replace(/^UScriptStruct'/, '')
      .replace(/'$/, '')
      .trim();

    const pkg = dt.Package || '';
    const rows = dt.Rows ? Object.keys(dt.Rows) : [];

    tables[tableName] = {
      name: tableName,
      struct_name: cleanStruct,
      package: pkg,
      count: rows.length,
      rows: rows,
    };

    totalRows += rows.length;
  } catch (err) {
    console.warn(`Failed to parse ${file}: ${err.message}`);
  }
}

const tableCount = Object.keys(tables).length;
console.log(`Successfully indexed ${tableCount} DataTables with ${totalRows} total rows.`);

if (!fs.existsSync(TARGET_DIR)) {
  fs.mkdirSync(TARGET_DIR, { recursive: true });
}

const canonicalFilename = 'Palworld_DataTables_24575825.json';
const canonicalPath = path.join(TARGET_DIR, canonicalFilename);
const manifestPath = path.join(TARGET_DIR, 'manifest.json');

const indexPayload = {
  total_tables: tableCount,
  total_rows: totalRows,
  tables: tables,
};

const jsonContent = JSON.stringify(indexPayload);
fs.writeFileSync(canonicalPath, jsonContent, 'utf8');

const crypto = await import('crypto');
const sha256 = crypto.createHash('sha256').update(jsonContent).digest('hex');
const fileSizeBytes = Buffer.byteLength(jsonContent, 'utf8');
const indexSizeKB = (fileSizeBytes / 1024).toFixed(1);

console.log(`Wrote ${canonicalPath} (${indexSizeKB} KB, SHA-256: ${sha256})`);

const manifestPayload = {
  schema_version: "1.0.0",
  latest_game_version: "v1.0.3",
  latest_steam_build_id: "24575825",
  updated_at: new Date().toISOString(),
  datatables: [
    {
      game_version: "v1.0.3",
      steam_build_id: "24575825",
      app_id: 1623730,
      datatables_filename: canonicalFilename,
      datatables_url: `https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/datatables/${canonicalFilename}`,
      sha256: sha256,
      file_size_bytes: fileSizeBytes,
      total_tables: tableCount,
      total_rows: totalRows,
      engine_version: "UE5.1.1",
      build_id: "Pal-5.1.1-0+++UE5+Release-5.1-c838a8ac",
      source: "Palworld Cooked DataTables",
      is_latest: true
    }
  ]
};

fs.writeFileSync(manifestPath, JSON.stringify(manifestPayload, null, 2), 'utf8');
console.log(`Wrote ${manifestPath}`);

console.log('DataTable indexing complete!');
