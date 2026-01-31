# sheer-to-gcal

Sync `https://reservations-sheer.jp/user/reservelog.php` reservations to Google Calendar.

## Features

- Create events for: `予約完了`, `レッスン済`
- Delete events for: `キャンセル` (only events previously created by this extension)
- Incremental: uses local state `key(rv00 or fingerprint) -> eventId` to avoid duplicates
- Fixed duration: 45 minutes
- Calendar: `primary`

## Install (unpacked)

1. Open Chrome: `chrome://extensions`
2. Enable "Developer mode"
3. Click "Load unpacked" and select:
   - `d:\repos\tampermonkey-scripts\chrome-extension\sheer-to-gcal`

## Connect Google

1. Open the extension options page
2. Click "Connect Google"

Notes:

- This uses `chrome.identity.getAuthToken()`.
- You must configure OAuth for the extension (otherwise you will see `Invalid OAuth2 Client ID.`).

### OAuth setup (required)

1. Go to Google Cloud Console
2. Create/select a project
3. Enable **Google Calendar API**
4. Configure **OAuth consent screen** (External is fine for personal use; add yourself as test user)
5. Create **Credentials** → **OAuth client ID**
   - Application type: **Chrome Extension**
   - Extension ID: use your installed extension ID
     - Example: `diaehcbkkabpcnlmmljmgoblcogejafc`
6. Copy the generated **Client ID** and set it in:
   - `chrome-extension/sheer-to-gcal/manifest.json` → `oauth2.client_id`
7. Reload the extension in `chrome://extensions`
8. Open options → "Connect Google" again

## Use

1. Go to `https://reservations-sheer.jp/user/reservelog.php`
2. Click "Sync this page"

Notes:

- This extension creates and uses a dedicated calendar named `SHEER Lessons`.
- For safety, it never reuses an existing calendar with the same name.

The top bar shows:

- `New`: reservations that will be created
- `Cancelled-to-delete`: synced reservations detected as cancelled and will be deleted

## Deduplication / consistency

This extension prevents duplicates by storing a mapping in `chrome.storage.local`:

- `key (rv00 or fingerprint) -> eventId`

On sync:

- If `eventId` exists in storage, the reservation is considered synced and will be updated (patched) to match the current payload.
  - Update is only applied when the existing event description contains `sheer_rv00=` (to avoid overwriting manual events).
- To avoid double-click duplicates, the sync button is locked while syncing and each reservation is marked as `in_flight` before calling `events.insert`.
- If you manually delete an event in Google Calendar, the next sync will detect it via `events.get` and will re-create it.
