# tampermonkey-scripts

Personal Tampermonkey userscripts.

## Scripts

| Script                              | Match                                                 | Install                                  |
| ----------------------------------- | ----------------------------------------------------- | ---------------------------------------- |
| Nico Commons ContentTree - Copy TSV | `https://commons.nicovideo.jp/works/*/tree/children*` | `dist/nico-commons-content-tree.user.js` |

## Dev

Requirements:

- Node >= 20

Commands:

- `npm test` (fixture based)
- `npm run build` (generate dist/\*.user.js)
- `npm run smoke:nico-commons -- <rootId>` (real API)
- `npm run format`
