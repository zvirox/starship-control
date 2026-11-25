import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { sanitizeMarkdownText } from "./sanitize.js";

test("script tags are stripped before render", () => {
  const payload = 'hello<script>alert("xss")</script>world';
  assert.equal(sanitizeMarkdownText(payload), "hello[script stripped]world");
});

test("javascript and data:text/html schemes are blocked", () => {
  for (const payload of ["javascript:alert(1)", "data:text/html,<script>alert(1)</script>"]) {
    assert.doesNotMatch(sanitizeMarkdownText(payload), /javascript:|data:text\/html/i);
  }
});

test("raw HTML stays inert as escaped text", () => {
  const payload = '<iframe src="javascript:alert(1)"></iframe>';
  const output = sanitizeMarkdownText(payload);
  assert.match(output, /&lt;iframe|<iframe/);
  assert.doesNotMatch(output, /javascript:/);
});

test("mutation: disabling the javascript scheme check makes a payload survive", async () => {
  const sourcePath = new URL("./sanitize.js", import.meta.url);
  const source = readFileSync(sourcePath, "utf8");
  const mutated = source
    .split("\n")
    .filter((line) => !line.includes("CHECK:javascript-scheme"))
    .filter((line) => !line.includes('.replace(/javascript\\s*:/gi'))
    .join("\n");

  assert.notEqual(mutated, source, "mutation target was not found");
  const directory = mkdtempSync(join(tmpdir(), "starship-mutation-"));
  const mutatedPath = join(directory, "sanitize.mutant.mjs");
  writeFileSync(mutatedPath, mutated);
  const mutant = await import(pathToFileURL(mutatedPath).href);

  assert.match(mutant.sanitizeMarkdownText("javascript:alert(1)"), /javascript:/);
  assert.doesNotMatch(sanitizeMarkdownText("javascript:alert(1)"), /javascript:/);
});
