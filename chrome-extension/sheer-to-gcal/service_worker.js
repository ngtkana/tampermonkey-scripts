const STORAGE_AUTH = "sheerToGcal.auth.v1";
const TOKEN_INFO_URL = "https://www.googleapis.com/oauth2/v3/tokeninfo";

const STORAGE_CALENDAR = "sheerToGcal.calendar.v1";
const MANAGED_MARKER = "managed_by=sheer-to-gcal";
const TARGET_CALENDAR_NAME = "SHEER Lessons";

console.log("sheer-to-gcal: service worker started", {
    id: chrome?.runtime?.id,
    manifestVersion: chrome?.runtime?.getManifest?.()?.manifest_version
});

self.addEventListener("error", (e) => {
    console.error("sheer-to-gcal: sw error", e?.message ?? e, e);
});

self.addEventListener("unhandledrejection", (e) => {
    console.error("sheer-to-gcal: sw unhandledrejection", e?.reason ?? e, e);
});

function nowSec() {
    return Math.floor(Date.now() / 1000);
}

async function getAuthState() {
    const res = await chrome.storage.local.get([STORAGE_AUTH]);
    return res[STORAGE_AUTH] ?? { accessToken: null, expiry: 0 };
}

async function setAuthState(state) {
    await chrome.storage.local.set({ [STORAGE_AUTH]: state });
}

async function clearAuthState() {
    await chrome.storage.local.remove([STORAGE_AUTH]);
}

async function getCalendarState() {
    const res = await chrome.storage.local.get([STORAGE_CALENDAR]);
    return res[STORAGE_CALENDAR] ?? null;
}

async function setCalendarState(state) {
    await chrome.storage.local.set({ [STORAGE_CALENDAR]: state });
}

async function tokenLooksValid(accessToken) {
    if (!accessToken) return false;
    try {
        const u = new URL(TOKEN_INFO_URL);
        u.searchParams.set("access_token", accessToken);
        const r = await fetch(u.toString());
        return r.ok;
    } catch {
        return false;
    }
}

async function ensureTokenInteractive() {
    // Use chrome.identity.getAuthToken with interactive=true.
    // Note: This requires the extension to have OAuth configured in the manifest for full reliability.
    // For personal use, we can rely on Chrome-managed token.
    const token = await chrome.identity.getAuthToken({ interactive: true });
    const accessToken = typeof token === "string" ? token : token?.token;
    const ok = await tokenLooksValid(accessToken);
    if (!ok) throw new Error("Failed to acquire valid access token");
    await setAuthState({ accessToken, expiry: nowSec() + 3000 });
    return accessToken;
}

async function getAccessToken({ interactive }) {
    const s = await getAuthState();
    if (s.accessToken && s.expiry && s.expiry - nowSec() > 60) return s.accessToken;

    if (!interactive) return null;
    return await ensureTokenInteractive();
}

async function gcalFetch(path, { method = "GET", body = null } = {}) {
    const accessToken = await getAccessToken({ interactive: true });
    const url = `https://www.googleapis.com/calendar/v3${path}`;
    const res = await fetch(url, {
        method,
        headers: {
            Authorization: `Bearer ${accessToken}`,
            "Content-Type": "application/json"
        },
        body: body ? JSON.stringify(body) : null
    });
    if (!res.ok) {
        const text = await res.text().catch(() => "");
        const err = new Error(`Google Calendar API error: ${res.status} ${res.statusText} ${text}`);
        err.status = res.status;
        err.statusText = res.statusText;
        err.bodyText = text;
        throw err;
    }
    if (res.status === 204) return null;
    return await res.json();
}

async function ensureManagedCalendar() {
    const existing = await getCalendarState();
    if (existing?.calendarId) return existing;

    // Safety-first: always create a new calendar. Never reuse an existing calendar with the same name.
    const created = await gcalFetch(`/calendars`, {
        method: "POST",
        body: {
            summary: TARGET_CALENDAR_NAME,
            description: MANAGED_MARKER
        }
    });

    const state = { calendarId: created.id, summary: created.summary };
    await setCalendarState(state);
    return state;
}

async function gcalInsertEvent(calendarId, event) {
    const data = await gcalFetch(`/calendars/${encodeURIComponent(calendarId)}/events`, {
        method: "POST",
        body: event
    });
    return { eventId: data.id };
}

async function gcalGetEvent(calendarId, eventId) {
    try {
        const data = await gcalFetch(`/calendars/${encodeURIComponent(calendarId)}/events/${encodeURIComponent(eventId)}`);
        return { ok: true, exists: true, eventId: data.id, description: data.description ?? "" };
    } catch (e) {
        if (e?.status === 404) {
            return { ok: true, exists: false };
        }
        throw e;
    }
}

async function gcalPatchEvent(calendarId, eventId, patch) {
    const data = await gcalFetch(`/calendars/${encodeURIComponent(calendarId)}/events/${encodeURIComponent(eventId)}`, {
        method: "PATCH",
        body: patch
    });
    return { ok: true, eventId: data.id };
}

async function gcalDeleteEvent(calendarId, eventId) {
    await gcalFetch(`/calendars/${encodeURIComponent(calendarId)}/events/${encodeURIComponent(eventId)}`, {
        method: "DELETE"
    });
    return { ok: true };
}

async function authStatus() {
    const s = await getAuthState();
    const authorized = !!(s.accessToken && (await tokenLooksValid(s.accessToken)));
    return { authorized };
}

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    (async () => {
        console.log("sheer-to-gcal: onMessage", msg?.type, msg);
        switch (msg?.type) {
            case "auth:status":
                sendResponse(await authStatus());
                return;
            case "auth:connect":
                await ensureTokenInteractive();
                sendResponse({ ok: true });
                return;
            case "auth:disconnect":
                await clearAuthState();
                try {
                    // Also invalidate cached token held by Chrome.
                    const token = await chrome.identity.getAuthToken({ interactive: false }).catch(() => null);
                    const accessToken = typeof token === "string" ? token : token?.token;
                    if (accessToken) {
                        await chrome.identity.removeCachedAuthToken({ token: accessToken });
                    }
                } catch {
                    // ignore
                }
                sendResponse({ ok: true });
                return;
            case "gcal:ensureCalendar":
                sendResponse(await ensureManagedCalendar());
                return;
            case "gcal:insert":
                sendResponse(await gcalInsertEvent(msg.calendarId, msg.event));
                return;
            case "gcal:get":
                sendResponse(await gcalGetEvent(msg.calendarId, msg.eventId));
                return;
            case "gcal:update":
                sendResponse(await gcalPatchEvent(msg.calendarId, msg.eventId, msg.patch));
                return;
            case "gcal:delete":
                sendResponse(await gcalDeleteEvent(msg.calendarId, msg.eventId));
                return;
            case "state:export": {
                const res = await chrome.storage.local.get(["sheerToGcal.state.v1"]);
                sendResponse({ state: res["sheerToGcal.state.v1"] ?? { eventsByKey: {} } });
                return;
            }
            case "state:import": {
                await chrome.storage.local.set({ "sheerToGcal.state.v1": msg.state });
                sendResponse({ ok: true });
                return;
            }
            case "state:reset": {
                await chrome.storage.local.set({ "sheerToGcal.state.v1": { eventsByKey: {} } });
                sendResponse({ ok: true });
                return;
            }
            default:
                sendResponse({ ok: false, error: "unknown message" });
        }
    })().catch((err) => {
        console.error("sheer-to-gcal: onMessage error", msg?.type, err);
        sendResponse({ ok: false, error: String(err?.message ?? err) });
    });
    return true;
});
