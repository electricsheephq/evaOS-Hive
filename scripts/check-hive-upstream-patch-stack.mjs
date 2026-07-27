import { readFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..");
const manifestPath = path.join(
  repositoryRoot,
  "docs",
  "hive-upstream-patch-stack.json",
);
const live = process.argv.includes("--live");
const allowedUpstreamDispositions = new Set([
  "keep",
  "redesign",
  "prove-inapplicable",
]);
const allowedElectricDispositions = new Set([
  "keep",
  "replay-narrowly",
  "redesign",
  "drop",
  "drop-merge-replay-children",
]);
const shaPattern = /^[0-9a-f]{40}$/;

function fail(message) {
  console.error(`Hive patch-stack check failed: ${message}`);
  process.exit(1);
}

function assert(condition, message) {
  if (!condition) {
    fail(message);
  }
}

function runGit(args, acceptedStatuses = [0]) {
  const result = spawnSync("git", args, {
    cwd: repositoryRoot,
    encoding: "utf8",
  });

  if (!acceptedStatuses.includes(result.status)) {
    fail(
      `git ${args.join(" ")} exited ${result.status}: ${result.stderr.trim()}`,
    );
  }

  return result;
}

function assertExactList(label, actual, expected) {
  assert(
    JSON.stringify(actual) === JSON.stringify(expected),
    `${label} differs.\nExpected: ${expected.join(", ")}\nActual: ${actual.join(", ")}`,
  );
}

function validateEntry(entry, allowedDispositions, label) {
  assert(entry && typeof entry === "object", `${label} must be an object`);
  assert(shaPattern.test(entry.sha), `${label} has an invalid SHA`);
  assert(
    typeof entry.subject === "string" && entry.subject.length > 0,
    `${label} is missing its subject`,
  );
  assert(
    allowedDispositions.has(entry.disposition),
    `${label} has unsupported disposition ${entry.disposition}`,
  );
}

const manifest = JSON.parse(await readFile(manifestPath, "utf8"));

assert(manifest.schemaVersion === 1, "unsupported schema version");
for (const [name, sha] of Object.entries({
  upstreamBase: manifest.refs.upstreamBase,
  upstreamTarget: manifest.refs.upstreamTarget,
  candidateHead: manifest.refs.candidateHead,
  resultTree: manifest.mergeRehearsal.resultTree,
})) {
  assert(shaPattern.test(sha), `${name} is not a full SHA`);
}

assert(
  manifest.upstream.length === manifest.expectedCounts.upstream,
  "upstream count does not match expectedCounts",
);
assert(
  manifest.electricFirstParent.length ===
    manifest.expectedCounts.electricFirstParent,
  "Electric first-parent count does not match expectedCounts",
);
assert(
  manifest.electricMergeChildren.length ===
    manifest.expectedCounts.electricMergeChildren,
  "Electric merge-child count does not match expectedCounts",
);

manifest.upstream.forEach((entry, index) =>
  validateEntry(entry, allowedUpstreamDispositions, `upstream[${index}]`),
);
manifest.electricFirstParent.forEach((entry, index) => {
  validateEntry(
    entry,
    allowedElectricDispositions,
    `electricFirstParent[${index}]`,
  );
  assert(
    typeof entry.purpose === "string" && entry.purpose.length > 0,
    `electricFirstParent[${index}] is missing its purpose`,
  );
});
manifest.electricMergeChildren.forEach((entry, index) => {
  validateEntry(
    entry,
    allowedElectricDispositions,
    `electricMergeChildren[${index}]`,
  );
  assert(
    shaPattern.test(entry.merge),
    `electricMergeChildren[${index}] has an invalid merge SHA`,
  );
  assert(
    typeof entry.purpose === "string" && entry.purpose.length > 0,
    `electricMergeChildren[${index}] is missing its purpose`,
  );
});

const allCommits = [
  ...manifest.upstream,
  ...manifest.electricFirstParent,
  ...manifest.electricMergeChildren,
].map((entry) => entry.sha);
assert(
  new Set(allCommits).size === allCommits.length,
  "a commit appears in more than one manifest list",
);

const mergeCommits = new Set(
  manifest.electricFirstParent
    .filter((entry) => entry.disposition === "drop-merge-replay-children")
    .map((entry) => entry.sha),
);
for (const child of manifest.electricMergeChildren) {
  assert(
    mergeCommits.has(child.merge),
    `merge child ${child.sha} names an untracked merge ${child.merge}`,
  );
}
for (const merge of mergeCommits) {
  assert(
    manifest.electricMergeChildren.some((entry) => entry.merge === merge),
    `merge ${merge} has no recorded semantic child`,
  );
}

const conflicts = manifest.mergeRehearsal.conflicts;
assert(
  conflicts.length > 0 && new Set(conflicts).size === conflicts.length,
  "merge conflict paths must be non-empty and unique",
);
assertExactList(
  "sorted conflict paths",
  [...conflicts].sort(),
  conflicts,
);

if (live) {
  const upstreamLog = runGit([
    "log",
    "--reverse",
    "--format=%H%x09%s",
    `${manifest.refs.upstreamBase}..${manifest.refs.upstreamTarget}`,
  ])
    .stdout.trim()
    .split("\n")
    .filter(Boolean);
  assertExactList(
    "upstream commit range",
    upstreamLog,
    manifest.upstream.map((entry) => `${entry.sha}\t${entry.subject}`),
  );

  const electricLog = runGit([
    "log",
    "--first-parent",
    "--reverse",
    "--format=%H%x09%s",
    `${manifest.refs.upstreamBase}..${manifest.refs.candidateHead}`,
  ])
    .stdout.trim()
    .split("\n")
    .filter(Boolean);
  assertExactList(
    "Electric first-parent range",
    electricLog,
    manifest.electricFirstParent.map(
      (entry) => `${entry.sha}\t${entry.subject}`,
    ),
  );

  for (const merge of mergeCommits) {
    const actualChildren = runGit([
      "log",
      "--reverse",
      "--format=%H%x09%s",
      `${merge}^1..${merge}^2`,
    ])
      .stdout.trim()
      .split("\n")
      .filter(Boolean);
    const expectedChildren = manifest.electricMergeChildren
      .filter((entry) => entry.merge === merge)
      .map((entry) => `${entry.sha}\t${entry.subject}`);
    assertExactList(
      `semantic children of merge ${merge}`,
      actualChildren,
      expectedChildren,
    );
  }

  const rehearsal = runGit(
    [
      "merge-tree",
      "--write-tree",
      "--messages",
      manifest.refs.candidateHead,
      manifest.refs.upstreamTarget,
    ],
    [manifest.mergeRehearsal.expectedExitCode],
  );
  const lines = rehearsal.stdout.trim().split("\n");
  assert(
    lines[0] === manifest.mergeRehearsal.resultTree,
    `merge rehearsal tree changed from ${manifest.mergeRehearsal.resultTree} to ${lines[0]}`,
  );
  const actualConflicts = [...rehearsal.stdout.matchAll(
    /^CONFLICT \([^)]+\): .* in (.+)$/gm,
  )]
    .map((match) => match[1].trim())
    .sort();
  assertExactList(
    "merge rehearsal conflicts",
    actualConflicts,
    [...conflicts].sort(),
  );
}

console.log(
  `Hive patch-stack manifest OK (${manifest.upstream.length} upstream, ` +
    `${manifest.electricFirstParent.length} Electric first-parent, ` +
    `${manifest.electricMergeChildren.length} merge-child commits` +
    `${live ? ", live rehearsal verified" : ", static verification"}).`,
);
