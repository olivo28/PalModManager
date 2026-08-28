import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..');

const dirsToClean = [
  'dist',
  'src-tauri/target/debug/incremental',
  'src-tauri/target-desktop',
];

console.log('🧹 [PalModManager] Cleaning build caches & temp directories...');

let totalCleaned = 0;

for (const dir of dirsToClean) {
  const fullPath = path.resolve(repoRoot, dir);
  if (fs.existsSync(fullPath)) {
    try {
      fs.rmSync(fullPath, { recursive: true, force: true });
      console.log(`  ✓ Removed: ${dir}`);
      totalCleaned++;
    } catch (err) {
      console.warn(`  ⚠️ Could not remove ${dir}:`, err.message);
    }
  }
}

console.log(`✨ Cleanup complete! Cleaned ${totalCleaned} cache directories.`);
