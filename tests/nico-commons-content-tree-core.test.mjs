import { describe, it, expect } from "vitest";
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const require = createRequire(import.meta.url);
const core = require("../lib/nico-commons-content-tree-core.cjs");

const childrenSample = JSON.parse(
  fs.readFileSync(path.join(__dirname, "fixtures", "nico-commons-children.sample.json"), "utf8")
);

describe("nico-commons-content-tree core", () => {
  it("buildChildrenApiUrl builds expected query params", () => {
    const url = core.buildChildrenApiUrl("sm9", { offset: 100, limit: 50 });
    expect(url).toContain("/v1/tree/sm9/relatives/children");
    expect(url).toContain("_offset=100");
    expect(url).toContain("_limit=50");
    expect(url).toContain("with_meta=1");
    expect(url).toContain("_sort=-id");
    expect(url).toContain("only_mine=0");
  });

  it("extractCandidates filters by sm + keyword + url", () => {
    const contents = childrenSample.data.children.contents;
    const candidates = core.extractCandidates(contents);
    expect(candidates.length).toBe(2);
    expect(candidates[0].url).toContain("sm123");
    expect(candidates[1].url).toContain("sm999");
  });

  it("buildTsv uses userMap names when present", () => {
    const contents = childrenSample.data.children.contents;
    const candidates = core.extractCandidates(contents);
    const map = new Map([
      [123, "alice"],
      [999, "bob"],
    ]);
    const tsv = core.buildTsv(candidates, map);
    const lines = tsv.split("\n");
    expect(lines).toHaveLength(2);
    expect(lines[0]).toBe("【歌ってみた】foo\talice\thttps://www.nicovideo.jp/watch/sm123");
    expect(lines[1]).toBe("歌わせていただき qux\tbob\thttps://www.nicovideo.jp/watch/sm999");
  });
});
