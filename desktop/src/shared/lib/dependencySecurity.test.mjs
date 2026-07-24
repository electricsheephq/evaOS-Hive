import assert from "node:assert/strict";
import { createRequire } from "node:module";
import test from "node:test";

const require = createRequire(import.meta.url);
const productionDependencyRequire = createRequire(
  require.resolve("tiptap-markdown"),
);
const LinkifyIt = productionDependencyRequire("linkify-it");
const MarkdownIt = productionDependencyRequire("markdown-it");

function assertCompletesWithin(name, operation, limitMs = 1_000) {
  const startedAt = performance.now();
  operation();
  const elapsedMs = performance.now() - startedAt;

  assert.ok(
    elapsedMs < limitMs,
    `${name} took ${Math.round(elapsedMs)}ms (limit ${limitMs}ms)`,
  );
}

test("production linkifier bounds repeated fuzzy-email and mailto scans", () => {
  const linkifier = new LinkifyIt();

  assertCompletesWithin("fuzzy-email scan", () =>
    linkifier.match("a@b.com\n".repeat(8_000)),
  );
  assertCompletesWithin("mailto scan", () =>
    linkifier.match("mailto:".repeat(32_000)),
  );
});

test("production markdown parser bounds smartquote replacement work", () => {
  const markdown = new MarkdownIt({ typographer: true });

  assertCompletesWithin("smartquote render", () =>
    markdown.render('"'.repeat(80_000)),
  );
});
