"use strict";

const core = require("../lib/nico-commons-content-tree-core.cjs");

async function main() {
  const rootId = process.argv[2];
  if (!rootId) {
    console.error("Usage: npm run smoke:nico-commons -- <rootId>");
    console.error("Example: npm run smoke:nico-commons -- sm9");
    process.exit(2);
  }

  const fetchImpl = globalThis.fetch;
  if (typeof fetchImpl !== "function") {
    console.error("global fetch is not available. Use Node 18+.");
    process.exit(2);
  }

  const children = await core.fetchAllChildren(fetchImpl, rootId);
  const candidates = core.extractCandidates(children);
  const userMap = await core.fetchUserMap(
    fetchImpl,
    candidates.map((x) => x.userId)
  );
  const tsv = core.buildTsv(candidates, userMap);

  console.log(`# rootId: ${rootId}`);
  console.log(`# children: ${children.length}`);
  console.log(`# candidates: ${candidates.length}`);
  console.log("");
  console.log(tsv);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
