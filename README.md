# tampermonkey-scripts

Personal Tampermonkey userscripts.

## Scripts

| Script                              | Match                                                 | Install                                                               |
| ----------------------------------- | ----------------------------------------------------- | --------------------------------------------------------------------- |
| Nico Commons ContentTree - Copy TSV | `https://commons.nicovideo.jp/works/*/tree/children*` | `scripts/nico-commons-content-tree/nico-commons-content-tree.user.js` |

## Dev

Requirements:

- Node >= 20

Commands:

- `npm test` (fixture based)
- `npm run smoke:nico-commons -- <rootId>` (real API)
- `npm run format`
