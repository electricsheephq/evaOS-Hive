#!/usr/bin/env node

import { createHash } from "node:crypto";
import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { spawn, spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const contractPath = join(
  repoRoot,
  "desktop/src-tauri/evaos-teams/package-contract.json",
);
const tauriConfigPath = join(
  repoRoot,
  "desktop/src-tauri/tauri.evaos-teams.conf.json",
);
const contract = JSON.parse(readFileSync(contractPath, "utf8"));
const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, "utf8"));

function fail(message) {
  throw new Error(message);
}

function parseArgs(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith("--") || value === undefined) {
      fail(
        "Usage: verify-evaos-teams-app-bundle.mjs --expected-sha SHA --app APP --dmg DMG --evidence PATH",
      );
    }
    values.set(key.slice(2), value);
  }
  for (const key of ["expected-sha", "app", "dmg", "evidence"]) {
    if (!values.has(key)) fail(`Missing --${key}`);
  }
  return Object.fromEntries(values);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    ...options,
  });
  if (result.status !== 0) {
    fail(`${command} ${args.join(" ")} failed with status ${result.status}`);
  }
  return result.stdout.trim();
}

function plistValue(plist, keyPath) {
  return run("plutil", ["-extract", keyPath, "raw", "-o", "-", plist]);
}

function sha256(path) {
  const hash = createHash("sha256");
  hash.update(readFileSync(path));
  return hash.digest("hex");
}

function requireFile(path, description) {
  if (!existsSync(path)) fail(`Missing ${description}: ${path}`);
  return realpathSync(path);
}

function requireArm64(path, description) {
  const architectures = run("lipo", ["-archs", path])
    .split(/\s+/)
    .filter(Boolean);
  if (architectures.length !== 1 || architectures[0] !== "arm64") {
    fail(
      `${description} must be arm64-only; found ${architectures.join(", ") || "none"}`,
    );
  }
}

async function coldStart(mainBinary, aliveMs) {
  const scratchRoot = process.env.RUNNER_TEMP || process.env.TMPDIR || tmpdir();
  const scratch = mkdtempSync(join(scratchRoot, "evaos-teams-cold-start-"));
  const child = spawn(mainBinary, [], {
    cwd: scratch,
    env: {
      ...process.env,
      HOME: scratch,
      TMPDIR: scratch,
      RUST_LOG: "off",
    },
    stdio: "ignore",
  });

  let earlyExit;
  const exited = new Promise((resolveExit) => {
    child.once("exit", (code, signal) => {
      earlyExit = { code, signal };
      resolveExit("exited");
    });
  });
  const stayedAlive = new Promise((resolveAlive) => {
    setTimeout(() => resolveAlive("alive"), aliveMs);
  });

  try {
    const result = await Promise.race([exited, stayedAlive]);
    if (result !== "alive") {
      fail(
        `Managed app exited before ${aliveMs}ms (code=${earlyExit?.code}, signal=${earlyExit?.signal})`,
      );
    }

    child.kill("SIGTERM");
    const terminated = await Promise.race([
      exited,
      new Promise((resolveTimeout) =>
        setTimeout(() => resolveTimeout("timeout"), 5_000),
      ),
    ]);
    if (terminated === "timeout") {
      child.kill("SIGKILL");
      await exited;
    }
  } finally {
    if (child.exitCode === null && child.signalCode === null) {
      child.kill("SIGKILL");
      await exited;
    }
    rmSync(scratch, { recursive: true, force: true });
  }
}

const args = parseArgs(process.argv.slice(2));
const expectedSha = args["expected-sha"];
if (!/^[0-9a-f]{40}$/.test(expectedSha)) {
  fail("--expected-sha must be a full lowercase 40-character Git SHA");
}
if (run("git", ["rev-parse", "HEAD"]) !== expectedSha) {
  fail("Checked-out source does not match --expected-sha");
}
if (run("git", ["status", "--porcelain"]) !== "") {
  fail("Source worktree must remain clean through bundle verification");
}

const appPath = requireFile(resolve(args.app), "managed app bundle");
const dmgPath = requireFile(resolve(args.dmg), "managed DMG");
if (basename(appPath) !== `${contract.productName}.app`) {
  fail(`Unexpected app name: ${basename(appPath)}`);
}
if (basename(dmgPath) !== contract.artifactName) {
  fail(`Unexpected DMG name: ${basename(dmgPath)}`);
}

const infoPlist = requireFile(
  join(appPath, "Contents/Info.plist"),
  "Info.plist",
);
for (const [keyPath, expected] of [
  ["CFBundleDisplayName", contract.productName],
  ["CFBundleName", contract.productName],
  ["CFBundleIdentifier", contract.bundleIdentifier],
  ["CFBundleShortVersionString", contract.version],
  ["CFBundleURLTypes.0.CFBundleURLSchemes.0", contract.deepLinkScheme],
]) {
  const actual = plistValue(infoPlist, keyPath);
  if (actual !== expected) {
    fail(`${keyPath} must be ${expected}; found ${actual}`);
  }
}

if (tauriConfig.bundle.createUpdaterArtifacts !== false) {
  fail("Managed config must disable updater artifacts");
}
if (
  !Array.isArray(tauriConfig.plugins?.updater?.endpoints) ||
  tauriConfig.plugins.updater.endpoints.length !== 0
) {
  fail("Managed config must contain zero updater endpoints");
}

const executablePaths = [
  [join(appPath, "Contents/MacOS", contract.productName), "main binary"],
  [join(appPath, "Contents/MacOS/buzz-acp"), "buzz-acp sidecar"],
  [join(appPath, "Contents/MacOS/buzz-agent"), "buzz-agent sidecar"],
  [join(appPath, "Contents/MacOS/buzz-dev-mcp"), "buzz-dev-mcp sidecar"],
  [
    join(appPath, "Contents/MacOS/git-credential-nostr"),
    "git-credential-nostr sidecar",
  ],
  [join(appPath, "Contents/MacOS/buzz"), "buzz CLI sidecar"],
].map(([path, description]) => [requireFile(path, description), description]);

for (const [path, description] of executablePaths) {
  requireArm64(path, description);
}

for (const [path, description] of [
  [
    join(appPath, "Contents/Resources/licenses/Buzz-Apache-2.0.txt"),
    "Apache license",
  ],
  [
    join(appPath, "Contents/Resources/licenses/evaOS-Teams-NOTICE.txt"),
    "evaOS Teams notice",
  ],
]) {
  requireFile(path, description);
}

const signature = spawnSync("codesign", ["-dvv", appPath], {
  cwd: repoRoot,
  encoding: "utf8",
});
const signatureText = `${signature.stdout || ""}\n${signature.stderr || ""}`;
if (
  /^Authority=/m.test(signatureText) ||
  /^TeamIdentifier=(?!not set)/m.test(signatureText)
) {
  fail("Pre-sign bundle unexpectedly contains an identity signature");
}
const signatureKind = /Signature=adhoc/.test(signatureText)
  ? "ad-hoc"
  : "unsigned";

const aliveMs = 8_000;
await coldStart(executablePaths[0][0], aliveMs);

const evidencePath = resolve(args.evidence);
mkdirSync(dirname(evidencePath), { recursive: true });
const evidence = {
  schemaVersion: 1,
  sourceSha: expectedSha,
  product: {
    name: contract.productName,
    version: contract.version,
    bundleIdentifier: contract.bundleIdentifier,
    deepLinkScheme: contract.deepLinkScheme,
    artifactName: contract.artifactName,
  },
  build: {
    target: "aarch64-apple-darwin",
    hostArchitecture: run("uname", ["-m"]),
    signatureKind,
    environmentPolicy: "explicit allowlist",
    updaterEndpoints: 0,
    updaterArtifacts: false,
  },
  functionalSmoke: {
    mode: "isolated signed-out cold start",
    aliveMilliseconds: aliveMs,
    packagedNetworkObservation: "not performed before signing",
  },
  sha256: Object.fromEntries([
    ["Cargo.lock", sha256(join(repoRoot, "Cargo.lock"))],
    ["pnpm-lock.yaml", sha256(join(repoRoot, "pnpm-lock.yaml"))],
    [
      "tauri.evaos-teams.conf.json",
      sha256(join(repoRoot, "desktop/src-tauri/tauri.evaos-teams.conf.json")),
    ],
    ["package-contract.json", sha256(contractPath)],
    ["managed-dmg", sha256(dmgPath)],
    ...executablePaths.map(([path, description]) => [
      description,
      sha256(path),
    ]),
  ]),
};
writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, {
  mode: 0o600,
});
console.log(
  `Verified ${contract.productName} ${contract.version} unsigned arm64 bundle from ${expectedSha}`,
);
console.log(`Evidence: ${evidencePath}`);
