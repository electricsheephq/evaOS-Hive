import { spawnSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..");
const manifestPath = path.join(
  repositoryRoot,
  "docs",
  "hive-upstream-v0.5.1-dispositions.json",
);

const expectedRefs = Object.freeze({
  upstreamBase: "dd222a509b156ba52ed3219e895d7bf1cf322c92",
  v050Tag: "v0.5.0",
  v050Commit: "4a977c588a540be38bd8ddb268cd24437bac8165",
  v051Tag: "v0.5.1",
  v051Commit: "a13085e9ac9a7c8dbd9426a6b88fc75abf62220e",
  candidate: "a13085e9ac9a7c8dbd9426a6b88fc75abf62220e",
});
const expectedCounts = Object.freeze({
  total: 104,
  baseToV050: 67,
  v050ToV051: 37,
});
const allowedDispositions = new Set([
  "keep/adopt",
  "managed-surface omit while retaining upstream",
  "redesign/narrow replay",
  "prove-inapplicable",
]);
const allowedSegments = new Set([
  "base-to-v0.5.0",
  "v0.5.0-to-v0.5.1",
]);
const shaPattern = /^[0-9a-f]{40}$/;

function fail(message) {
  console.error(`Hive v0.5.1 disposition check failed: ${message}`);
  process.exit(1);
}

function assert(condition, message) {
  if (!condition) {
    fail(message);
  }
}

function runGit(args) {
  const result = spawnSync("git", args, {
    cwd: repositoryRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    fail(
      `git ${args.join(" ")} exited ${result.status}: ${result.stderr.trim()}`,
    );
  }
  return result.stdout.trim();
}

function assertExact(label, actual, expected) {
  assert(
    actual === expected,
    `${label} differs. Expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
  );
}

function readOrderedRange(from, to) {
  const output = runGit([
    "log",
    "--reverse",
    "--format=%H%x09%s",
    `${from}..${to}`,
  ]);
  return output ? output.split("\n") : [];
}

const manifest = JSON.parse(await readFile(manifestPath, "utf8"));

assert(manifest.schemaVersion === 1, "unsupported schema version");
for (const [name, expected] of Object.entries(expectedRefs)) {
  assertExact(`manifest ref ${name}`, manifest.refs?.[name], expected);
}
for (const [name, expected] of Object.entries(expectedCounts)) {
  assertExact(
    `manifest expected count ${name}`,
    manifest.expectedCounts?.[name],
    expected,
  );
}
assert(
  JSON.stringify(manifest.allowedDispositions) ===
    JSON.stringify([...allowedDispositions]),
  "allowed disposition list drifted",
);
assert(
  typeof manifest.proofBoundary === "string" &&
    manifest.proofBoundary.includes("Planning and source-history proof only") &&
    manifest.proofBoundary.includes("does not prove"),
  "proof boundary is missing or weakened",
);

assertExact(
  "pinned base identity",
  runGit(["rev-parse", `${expectedRefs.upstreamBase}^{commit}`]),
  expectedRefs.upstreamBase,
);
assertExact(
  "v0.5.0 tag identity",
  runGit(["rev-parse", `${expectedRefs.v050Tag}^{commit}`]),
  expectedRefs.v050Commit,
);
assertExact(
  "v0.5.1 tag identity",
  runGit(["rev-parse", `${expectedRefs.v051Tag}^{commit}`]),
  expectedRefs.v051Commit,
);
for (const [older, newer, label] of [
  [expectedRefs.upstreamBase, expectedRefs.v050Commit, "base -> v0.5.0"],
  [expectedRefs.v050Commit, expectedRefs.v051Commit, "v0.5.0 -> v0.5.1"],
]) {
  const result = spawnSync("git", ["merge-base", "--is-ancestor", older, newer], {
    cwd: repositoryRoot,
    encoding: "utf8",
  });
  assert(result.status === 0, `${label} ancestry check failed`);
}

assert(Array.isArray(manifest.commits), "commits must be an array");
assertExact("total commit count", manifest.commits.length, expectedCounts.total);

const seen = new Set();
manifest.commits.forEach((entry, index) => {
  const label = `commits[${index}]`;
  assert(entry && typeof entry === "object", `${label} must be an object`);
  assertExact(`${label} ordinal`, entry.ordinal, index + 1);
  assert(shaPattern.test(entry.sha), `${label} has an invalid full SHA`);
  assert(!seen.has(entry.sha), `${label} duplicates commit ${entry.sha}`);
  seen.add(entry.sha);
  assert(
    typeof entry.subject === "string" && entry.subject.length > 0,
    `${label} is missing its exact subject`,
  );
  assert(
    allowedSegments.has(entry.segment),
    `${label} has unsupported segment ${entry.segment}`,
  );
  assert(
    allowedDispositions.has(entry.disposition),
    `${label} has unsupported disposition ${entry.disposition}`,
  );
  assert(
    typeof entry.rationale === "string" && entry.rationale.length > 0,
    `${label} is missing its rationale`,
  );
  assert(
    typeof entry.risk === "string" && entry.risk.length > 0,
    `${label} is missing its risk`,
  );
});

const baseToV050 = readOrderedRange(
  expectedRefs.upstreamBase,
  expectedRefs.v050Commit,
);
const v050ToV051 = readOrderedRange(
  expectedRefs.v050Commit,
  expectedRefs.v051Commit,
);
const fullRange = readOrderedRange(
  expectedRefs.upstreamBase,
  expectedRefs.v051Commit,
);
assertExact("base-to-v0.5.0 count", baseToV050.length, expectedCounts.baseToV050);
assertExact("v0.5.0-to-v0.5.1 count", v050ToV051.length, expectedCounts.v050ToV051);
assertExact("full range count", fullRange.length, expectedCounts.total);

const recorded = manifest.commits.map(
  (entry) => `${entry.sha}\t${entry.subject}`,
);
assert(
  JSON.stringify(recorded) === JSON.stringify(fullRange),
  "ordered commit hashes or exact subjects drifted from Git history",
);
assert(
  JSON.stringify(recorded.slice(0, expectedCounts.baseToV050)) ===
    JSON.stringify(baseToV050),
  "first 67 entries do not exactly match base-to-v0.5.0 history",
);
assert(
  JSON.stringify(recorded.slice(expectedCounts.baseToV050)) ===
    JSON.stringify(v050ToV051),
  "last 37 entries do not exactly match v0.5.0-to-v0.5.1 history",
);
manifest.commits.forEach((entry, index) => {
  const expectedSegment =
    index < expectedCounts.baseToV050
      ? "base-to-v0.5.0"
      : "v0.5.0-to-v0.5.1";
  assertExact(`commits[${index}] segment`, entry.segment, expectedSegment);
});

const actualTotals = Object.fromEntries(
  [...allowedDispositions].map((disposition) => [
    disposition,
    manifest.commits.filter((entry) => entry.disposition === disposition).length,
  ]),
);
assert(
  JSON.stringify(manifest.dispositionTotals) === JSON.stringify(actualTotals),
  "disposition totals do not match commit entries",
);

console.log(
  "Hive v0.5.1 disposition manifest OK " +
    `(${expectedCounts.total} unique commits: ` +
    `${expectedCounts.baseToV050} base-to-v0.5.0 + ` +
    `${expectedCounts.v050ToV051} v0.5.0-to-v0.5.1).`,
);

