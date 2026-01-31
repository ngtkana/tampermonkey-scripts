# tampermonkey-scripts

## SHEER reservlog -> Google Calendar (no OAuth)

Build:

```bash
npm run build
```

Output:

- `dist/sheer-reservelog-to-gcal-url.user.js`

Install the built userscript into Tampermonkey, then visit:

- `https://reservations-sheer.jp/user/reservelog.php`

It adds `Add` / `Add all (new)` buttons that open Google Calendar's event creation page with fields pre-filled.


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
