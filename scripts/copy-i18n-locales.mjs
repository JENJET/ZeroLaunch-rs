import { copyFile, mkdir, readdir } from "node:fs/promises";
import { join } from "node:path";
import process from "node:process";

const rootDir = process.cwd();
const srcLocalesDir = join(rootDir, "src-ui", "i18n", "locales");
const destDir = join(rootDir, "src-tauri", "locales");

await mkdir(destDir, { recursive: true });

const entries = await readdir(srcLocalesDir, { withFileTypes: true });
const files = entries.filter((entry) => entry.isFile());

await Promise.all(
  files.map((file) =>
    copyFile(join(srcLocalesDir, file.name), join(destDir, file.name))
  )
);

console.log(`✓ ${files.length} i18n locales files copied to src-tauri/locales/`);
