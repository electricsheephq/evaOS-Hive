import { existsSync, readdirSync, renameSync, statSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import contract from "../src-tauri/evaos-teams/package-contract.json" with {
  type: "json",
};

for (const name of ["BUZZ_UPDATER_PUBLIC_KEY", "BUZZ_UPDATER_ENDPOINT"]) {
  if (process.env[name]?.trim()) {
    throw new Error(`${name} must not be set for Hive managed builds`);
  }
}

const desktopDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const result = spawnSync(
  "pnpm",
  [
    "exec",
    "tauri",
    "build",
    "--features",
    "evaos-teams-managed",
    "--target",
    "aarch64-apple-darwin",
    "--bundles",
    "app,dmg",
    "--config",
    "src-tauri/tauri.evaos-teams.conf.json",
    ...process.argv.slice(2),
  ],
  {
    cwd: desktopDir,
    env: process.env,
    stdio: "inherit",
  },
);
if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

function findDmgs(directory) {
  if (!existsSync(directory)) return [];
  const matches = [];
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) {
      matches.push(...findDmgs(path));
    } else if (entry.endsWith(".dmg")) {
      matches.push(path);
    }
  }
  return matches;
}

const bundleRoot = resolve(
  desktopDir,
  "src-tauri/target/aarch64-apple-darwin/release/bundle",
);
const dmgs = findDmgs(bundleRoot);
if (dmgs.length !== 1) {
  throw new Error(
    `Expected exactly one managed DMG under ${bundleRoot}, found ${dmgs.length}`,
  );
}
const artifactPath = join(dirname(dmgs[0]), contract.artifactName);
if (dmgs[0] !== artifactPath) {
  renameSync(dmgs[0], artifactPath);
}
console.log(artifactPath);
