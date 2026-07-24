import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const workspace = readFileSync(
  path.join(repoRoot, "pnpm-workspace.yaml"),
  "utf8",
);
const pnpmLock = readFileSync(path.join(repoRoot, "pnpm-lock.yaml"), "utf8");
const cargoLock = readFileSync(path.join(repoRoot, "Cargo.lock"), "utf8");

const javascriptFloors = [
  ["@babel/core", "7.29.6"],
  ["linkify-it", "5.0.2"],
  ["markdown-it", "14.2.0"],
];

for (const [name, version] of javascriptFloors) {
  assert.match(
    workspace,
    new RegExp(
      `^[ ]{2}["']?${name.replaceAll("/", "\\/")}["']?: ${version}$`,
      "m",
    ),
    `${name} must remain pinned to its reviewed security floor`,
  );
  assert.match(
    pnpmLock,
    new RegExp(
      `^[ ]{2}['"]?${name.replaceAll("/", "\\/")}@${version}['"]?:$`,
      "m",
    ),
    `${name}@${version} must be resolved in pnpm-lock.yaml`,
  );
}

assert.doesNotMatch(
  pnpmLock,
  /^ {2}['"]?linkify-it@5\.0\.[01]['"]?:$/m,
  "pnpm-lock.yaml must not contain an affected linkify-it release",
);
assert.doesNotMatch(
  pnpmLock,
  /^ {2}['"]?markdown-it@14\.1\.1['"]?:$/m,
  "pnpm-lock.yaml must not contain the affected markdown-it release",
);
assert.doesNotMatch(
  pnpmLock,
  /^ {2}['"]?@babel\/core@7\.29\.0['"]?:$/m,
  "pnpm-lock.yaml must not contain the affected @babel/core release",
);

const cmovVersions = [
  ...cargoLock.matchAll(/\[\[package\]\]\nname = "cmov"\nversion = "([^"]+)"/g),
].map((match) => match[1]);

assert.deepEqual(
  cmovVersions,
  ["0.5.4"],
  "the release workspace must resolve only the patched cmov 0.5.4",
);

console.log("canary dependency contract passed");
