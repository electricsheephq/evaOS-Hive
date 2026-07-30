import assert from "node:assert/strict";
import { readFileSync, statSync } from "node:fs";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const contract = JSON.parse(
  readFileSync(`${root}src-tauri/hive/package-contract.json`, "utf8"),
);
const config = JSON.parse(
  readFileSync(`${root}src-tauri/tauri.hive.conf.json`, "utf8"),
);
const buildScript = readFileSync(`${root}scripts/build-hive.mjs`, "utf8");

test("Hive package identity and selected assets are coherent", () => {
  assert.equal(contract.productName, "Hive");
  assert.equal(contract.version, "0.5.2-es.1");
  assert.equal(contract.artifactName, "Hive-0.5.2-es.1-arm64.dmg");
  assert.equal(config.productName, contract.productName);
  assert.equal(config.version, contract.version);
  assert.equal(config.identifier, contract.bundleIdentifier);
  assert.equal(config.mainBinaryName, "Hive");
  assert.deepEqual(config.plugins["deep-link"].desktop.schemes, ["buzz"]);
  assert.ok(buildScript.includes("evaos-teams-managed"));
  assert.ok(buildScript.includes("tauri.hive.conf.json"));
  assert.match(buildScript, /HIVE_SUPABASE_PUBLISHABLE_KEY/);
  for (const path of [
    `${root}src-tauri/hive/icon.icns`,
    `${root}src-tauri/hive/icon.png`,
    `${root}src-tauri/hive/dmg-background.png`,
    `${root}public/hive-icon.png`,
  ]) {
    assert.ok(statSync(path).size > 0, `${path} must be non-empty`);
  }
});

test("Hive updater contract names only the signed fork channel", () => {
  assert.equal(contract.updateChannel, "hive-internal");
  assert.equal(
    contract.updateEndpoint,
    "https://github.com/electricsheephq/evaOS-Hive/releases/download/hive-desktop-latest/latest.json",
  );
  assert.deepEqual(config.plugins.updater.endpoints, []);
  assert.equal(config.bundle.createUpdaterArtifacts, false);
  assert.match(buildScript, /HIVE_UPDATER_PUBLIC_KEY/);
  assert.match(buildScript, /BUZZ_UPDATER_PUBLIC_KEY.*must not be set/s);
});
