import assert from "node:assert/strict";
import { createHash } from "node:crypto";
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
const sha256 = (contents) =>
  createHash("sha256").update(contents).digest("hex");

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
  assert.match(contract.artifactName, /^Hive-.+-arm64\.dmg$/);
  assert.equal(contract.updateChannel, "managed-beta");
});

test("managed package uses the approved Hive identity and Beekeeper icons", async () => {
  assert.equal(contract.productName, "Hive");
  assert.deepEqual(tauriConfig.bundle.icon, [
    "evaos-teams/icon.png",
    "evaos-teams/icon.icns",
  ]);
  assert.equal(
    sha256(
      await readFile(
        new URL("../../../src-tauri/evaos-teams/icon.png", import.meta.url),
      ),
    ),
    "e18c5e7ead889e26aa111921e9482b25eff91829b2a4cb9d854069ab2ccded0b",
  );
  assert.equal(
    sha256(
      await readFile(
        new URL("../../../src-tauri/evaos-teams/icon.icns", import.meta.url),
      ),
    ),
    "f5a9c0ca71034505d8c85e73dd4a77bbb272fbeaac717dee6d2680a360825b7c",
  );
  assert.equal(
    sha256(
      await readFile(
        new URL("../../../public/evaos-teams-icon.png", import.meta.url),
      ),
    ),
    "e18c5e7ead889e26aa111921e9482b25eff91829b2a4cb9d854069ab2ccded0b",
  );
});

test("managed bundle retains license and notice resources", () => {
  assert.equal(
    tauriConfig.bundle.resources["../../LICENSE"],
    "licenses/Buzz-Apache-2.0.txt",
  );
  assert.equal(
    tauriConfig.bundle.resources["evaos-teams/NOTICE.txt"],
    "licenses/Hive-NOTICE.txt",
  );
});
