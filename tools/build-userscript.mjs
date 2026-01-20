import esbuild from "esbuild";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function buildMetaBlock({ version, downloadURL, updateURL }) {
  const lines = [
    "// ==UserScript==",
    "// @name         Nico Commons ContentTree - Copy TSV",
    "// @namespace    https://ngtkana.local/",
    `// @version      ${version}`,
    "// @description  Nico Commons content tree children -> filter utaite covers -> copy TSV",
    "// @match        https://commons.nicovideo.jp/works/*/tree/children*",
    "// @grant        none",
    `// @downloadURL  ${downloadURL}`,
    `// @updateURL    ${updateURL}`,
    "// ==/UserScript==",
    "",
  ];
  return lines.join("\n");
}

async function main() {
  const rootDir = path.join(__dirname, "..");
  const pkg = readJson(path.join(rootDir, "package.json"));
  const version = pkg.version;

  const repo = process.env.GITHUB_REPOSITORY ?? "ngtkana/tampermonkey-scripts";
  const branch = process.env.USERSCRIPT_BRANCH ?? "main";
  const distName = "nico-commons-content-tree.user.js";
  const rawBase = `https://raw.githubusercontent.com/${repo}/${branch}`;
  const downloadURL = `${rawBase}/dist/${distName}`;
  const updateURL = downloadURL;

  const banner = buildMetaBlock({ version, downloadURL, updateURL });

  await esbuild.build({
    entryPoints: [path.join(rootDir, "src", "nico-commons-content-tree", "index.js")],
    bundle: true,
    format: "iife",
    platform: "browser",
    target: ["chrome109", "firefox109"],
    outfile: path.join(rootDir, "dist", distName),
    banner: {
      js: banner,
    },
  });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
