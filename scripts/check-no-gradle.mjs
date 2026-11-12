import { readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const root = process.cwd();
const skipDirectories = new Set([
  "node_modules",
  ".git",
  "target",
  "dist",
  ".turbo",
  ".expo",
  "build",
]);
const bannedFile =
  /^(gradlew|gradlew\.bat|gradle\.properties|settings\.gradle|settings\.gradle\.kts|[^/\\]+\.gradle|[^/\\]+\.gradle\.kts)$/i;
const bannedDirectory = /(^|[/\\])(\.gradle|gradle)([/\\]|$)/i;

const offenders = [];

function walk(directory) {
  for (const entry of readdirSync(directory)) {
    if (skipDirectories.has(entry)) continue;

    const fullPath = join(directory, entry);
    const stats = statSync(fullPath);
    const relativePath = relative(root, fullPath);

    if (bannedDirectory.test(relativePath)) {
      offenders.push(relativePath);
      continue;
    }

    if (stats.isDirectory()) {
      walk(fullPath);
    } else if (bannedFile.test(entry)) {
      offenders.push(relativePath);
    }
  }
}

walk(root);

if (offenders.length > 0) {
  console.error("Gradle artifacts found. This project builds native code with Cargo:");
  for (const offender of offenders) console.error(`  ${offender}`);
  process.exit(1);
}

console.log("No Gradle files or directories found.");
