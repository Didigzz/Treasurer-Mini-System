import {
  chmodSync,
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
} from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";

const desktopDir = resolve(import.meta.dir, "..");
const repoRoot = resolve(desktopDir, "..", "..");
const serverDir = join(repoRoot, "apps", "server");
const binariesDir = join(desktopDir, "src-tauri", "binaries");
const nativeModulesDir = join(binariesDir, "native_modules");
const executableExtension = process.platform === "win32" ? ".exe" : "";

function nativeLibsqlPackage(): string {
  const key = `${process.platform}-${process.arch}`;
  const packages: Record<string, string> = {
    "win32-x64": "win32-x64-msvc",
    "win32-arm64": "win32-arm64-msvc",
    "darwin-x64": "darwin-x64",
    "darwin-arm64": "darwin-arm64",
    "linux-x64": "linux-x64-gnu",
    "linux-arm64": "linux-arm64-gnu",
  };
  const name = packages[key];
  if (!name) throw new Error(`No libsql native package mapping for ${key}.`);
  return name;
}

function stageLibsqlNativeModule(): void {
  const requireFromServer = createRequire(join(serverDir, "package.json"));
  const libsqlPath = requireFromServer.resolve("libsql");
  const requireFromLibsql = createRequire(libsqlPath);
  const packageName = nativeLibsqlPackage();
  const nativeIndex = requireFromLibsql.resolve(`@libsql/${packageName}`);
  const nativePackageDir = dirname(nativeIndex);
  const destination = join(nativeModulesDir, "@libsql", packageName);

  cpSync(nativePackageDir, destination, { recursive: true });
  console.log(`Staged libsql native addon: ${destination}`);
}

function hostTriple(): string {
  const result = Bun.spawnSync(["rustc", "-vV"], { stdout: "pipe", stderr: "pipe" });
  if (result.exitCode !== 0) {
    throw new Error(
      `rustc -vV failed. Install Rust before building the desktop app.\n${result.stderr.toString()}`,
    );
  }

  const match = /^host:\s*(\S+)$/m.exec(result.stdout.toString());
  if (!match?.[1]) throw new Error("Could not determine the Rust host target triple.");
  return match[1];
}

const triple = hostTriple();

console.log("Compiling the Bun server sidecar…");
const compile = Bun.spawnSync(["bun", "run", "compile"], {
  cwd: serverDir,
  stdout: "inherit",
  stderr: "inherit",
});

if (compile.exitCode !== 0) {
  throw new Error(`bun run compile exited with code ${compile.exitCode}.`);
}

const builtBinary = join(serverDir, `server${executableExtension}`);
if (!existsSync(builtBinary)) {
  throw new Error(`Expected the compiled server at ${builtBinary}.`);
}

mkdirSync(binariesDir, { recursive: true });
const destination = join(binariesDir, `server-${triple}${executableExtension}`);
copyFileSync(builtBinary, destination);
chmodSync(destination, 0o755);
stageLibsqlNativeModule();

console.log(`Sidecar ready: ${destination}`);
