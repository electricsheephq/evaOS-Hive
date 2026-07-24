import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const tauriConfig = JSON.parse(
  await readFile(
    new URL("../../../src-tauri/tauri.evaos-teams.conf.json", import.meta.url),
    "utf8",
  ),
);
const contract = JSON.parse(
  await readFile(
    new URL(
      "../../../src-tauri/evaos-teams/package-contract.json",
      import.meta.url,
    ),
    "utf8",
  ),
);

test("managed package config matches the exact source contract", () => {
  assert.equal(tauriConfig.productName, contract.productName);
  assert.equal(tauriConfig.mainBinaryName, contract.productName);
  assert.equal(tauriConfig.version, contract.version);
  assert.equal(tauriConfig.identifier, contract.bundleIdentifier);
  assert.deepEqual(tauriConfig.plugins["deep-link"].desktop.schemes, [
    contract.deepLinkScheme,
  ]);
  assert.deepEqual(tauriConfig.plugins.updater.endpoints, []);
  assert.equal(tauriConfig.bundle.createUpdaterArtifacts, false);
  assert.match(contract.artifactName, /^evaOS-Teams-.+-arm64\.dmg$/);
  assert.equal(contract.updateChannel, "managed-beta");
});

test("managed bundle retains license and notice resources", () => {
  assert.equal(
    tauriConfig.bundle.resources["../../LICENSE"],
    "licenses/Buzz-Apache-2.0.txt",
  );
  assert.equal(
    tauriConfig.bundle.resources["evaos-teams/NOTICE.txt"],
    "licenses/evaOS-Teams-NOTICE.txt",
  );
});
