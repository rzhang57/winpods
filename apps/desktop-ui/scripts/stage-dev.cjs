const fs = require("fs");
const path = require("path");

const appRoot = process.cwd();
const repoRoot = path.resolve(appRoot, "../..");
const backendSource = path.join(repoRoot, "target", "debug", "desktop.exe");
const runtimeDir = path.join(appRoot, ".runtime", "backend");
const backendTarget = path.join(runtimeDir, "desktop.exe");

if (!fs.existsSync(backendSource)) {
  throw new Error(`Backend binary not found at: ${backendSource}`);
}

fs.mkdirSync(runtimeDir, { recursive: true });
fs.copyFileSync(backendSource, backendTarget);

console.log(`[stage] backend copied to ${backendTarget}`);
