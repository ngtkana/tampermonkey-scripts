const LESSON_MINUTES = 45;
const STORAGE_KEY = "sheerToGcal.state.v1";
const CALENDAR_KEY = "sheerToGcal.calendar.v1";
const MANAGED_MARKER = "managed_by=sheer-to-gcal";

let isSyncRunning = false;

function pad2(n) {
    return String(n).padStart(2, "0");
}

function formatLocalDateTimeForGCal(date) {
    // Google Calendar API expects RFC3339 with timezone.
    // We will send ISO with local offset.
    const tzOffsetMin = -date.getTimezoneOffset();
    const sign = tzOffsetMin >= 0 ? "+" : "-";
    const abs = Math.abs(tzOffsetMin);
    const hh = pad2(Math.floor(abs / 60));
    const mm = pad2(abs % 60);
    return (
        `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}` +
        `T${pad2(date.getHours())}:${pad2(date.getMinutes())}:00${sign}${hh}:${mm}`
    );
}

function addMinutes(date, minutes) {
    return new Date(date.getTime() + minutes * 60 * 1000);
}

function normalizeSpace(s) {
    return s.replace(/\s+/g, " ").trim();
}

function parseDateTimeFromCell(td) {
    // Example: "2026-02-22<br>15:00"
    const text = normalizeSpace(td.innerText);
    const m = text.match(/(\d{4}-\d{2}-\d{2})\s+(\d{2}:\d{2})/);
    if (!m) return null;
    const [_, d, t] = m;
    // Interpret as local time.
    const [Y, M, D] = d.split("-").map(Number);
    const [hh, mm] = t.split(":").map(Number);
    return new Date(Y, M - 1, D, hh, mm, 0, 0);
}

function extractRv00FromTd(td) {
    const a = td.querySelector("a[href*='rv00=']");
    if (!a) return null;
    try {
        const u = new URL(a.getAttribute("href"), location.href);
        return u.searchParams.get("rv00");
    } catch {
        const m = a.getAttribute("href").match(/(?:\?|&)rv00=(\d+)/);
        return m ? m[1] : null;
    }
}

function getStateLabel(td) {
    const text = normalizeSpace(td.innerText);
    if (text.startsWith("予約完了")) return "scheduled";
    if (text.startsWith("レッスン済")) return "done";
    if (text.startsWith("キャンセル")) return "cancelled";
    return "unknown";
}

function computeFingerprint({ start, studio, lesson, teacher }) {
    const d = `${start.getFullYear()}-${pad2(start.getMonth() + 1)}-${pad2(start.getDate())}`;
    const t = `${pad2(start.getHours())}:${pad2(start.getMinutes())}`;
    return `${d} ${t}|${studio}|${lesson}|${teacher}`;
}

function buildReservationFromRow(tr) {
    const tds = tr.querySelectorAll("td");
    if (tds.length < 5) return null;
    const start = parseDateTimeFromCell(tds[0]);
    if (!start) return null;
    const studio = normalizeSpace(tds[1].innerText);
    const lesson = normalizeSpace(tds[2].innerText);
    const teacher = normalizeSpace(tds[3].innerText);
    const state = getStateLabel(tds[4]);
    const rv00 = extractRv00FromTd(tds[4]);
    const key = rv00 ?? computeFingerprint({ start, studio, lesson, teacher });
    return { key, rv00, start, studio, lesson, teacher, state };
}

async function getSyncState() {
    const res = await chrome.storage.local.get([STORAGE_KEY]);
    return res[STORAGE_KEY] ?? { eventsByKey: {} };
}

async function setSyncState(state) {
    await chrome.storage.local.set({ [STORAGE_KEY]: state });
}

async function getCalendarId() {
    const res = await chrome.storage.local.get([CALENDAR_KEY]);
    const cached = res[CALENDAR_KEY];
    if (cached?.calendarId) return cached.calendarId;
    const created = await chrome.runtime.sendMessage({ type: "gcal:ensureCalendar" });
    // service worker already stores it, but cache here too for quick access.
    await chrome.storage.local.set({ [CALENDAR_KEY]: created });
    return created.calendarId;
}

function ensureTopBar() {
    let bar = document.getElementById("sheer-to-gcal-bar");
    if (bar) return bar;
    bar = document.createElement("div");
    bar.id = "sheer-to-gcal-bar";
    bar.style.cssText = [
        "position: sticky",
        "top: 0",
        "z-index: 9999",
        "background: #fff",
        "border: 1px solid #ddd",
        "padding: 8px",
        "margin-bottom: 8px",
        "font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif",
        "display: flex",
        "gap: 8px",
        "align-items: center"
    ].join(";");
    bar.innerHTML = `
    <button id="sheer-to-gcal-sync">Sync this page</button>
    <span id="sheer-to-gcal-summary" style="color:#333"></span>
    <span id="sheer-to-gcal-auth" style="color:#666"></span>
  `;
    const main = document.querySelector("#Main") ?? document.body;
    main.prepend(bar);
    return bar;
}

async function updateSummary() {
    const bar = ensureTopBar();
    const auth = await chrome.runtime.sendMessage({ type: "auth:status" });
    bar.querySelector("#sheer-to-gcal-auth").textContent = auth?.authorized
        ? "Connected"
        : "Not connected (open extension options to connect)";

    const state = await getSyncState();
    const rows = Array.from(document.querySelectorAll(".Reserve0Table tbody tr"))
        .map((tr) => ({ tr, r: buildReservationFromRow(tr) }))
        .filter(({ r }) => r);
    const candidates = rows.filter(({ r }) => r.state === "scheduled" || r.state === "done");
    const cancelled = rows.filter(({ r }) => r.state === "cancelled");
    const newCount = candidates.filter(({ r }) => !state.eventsByKey[r.key]).length;
    const cancelledKnown = cancelled.filter(({ r }) => state.eventsByKey[r.key]).length;
    bar.querySelector("#sheer-to-gcal-summary").textContent =
        `New: ${newCount} | Cancelled-to-delete: ${cancelledKnown}`;
}

async function reconcileCancelled(state, reservations) {
    const calendarId = await getCalendarId();
    for (const r of reservations.filter((x) => x.state === "cancelled")) {
        const known = state.eventsByKey[r.key];
        if (!known?.eventId) continue;
        const got = await chrome.runtime.sendMessage({ type: "gcal:get", calendarId, eventId: known.eventId });
        const desc = String(got?.description ?? "");
        if (desc.includes("sheer_rv00=") || desc.includes(MANAGED_MARKER)) {
            await chrome.runtime.sendMessage({ type: "gcal:delete", calendarId, eventId: known.eventId });
        }
        delete state.eventsByKey[r.key];
    }
}

async function reconcileMissingEvents(state, reservations) {
    const calendarId = await getCalendarId();
    for (const r of reservations.filter((x) => x.state === "scheduled" || x.state === "done")) {
        const known = state.eventsByKey[r.key];
        if (!known?.eventId) continue;
        const res = await chrome.runtime.sendMessage({ type: "gcal:get", calendarId, eventId: known.eventId });
        if (res?.exists === false) {
            delete state.eventsByKey[r.key];
        }
    }
}

function buildEventPayload(r) {
    const start = r.start;
    const end = addMinutes(start, LESSON_MINUTES);

    const summary = `SHEER ${r.studio} ${r.lesson} ${r.teacher}`;
    const detailsLines = [
        MANAGED_MARKER,
        `studio=${r.studio}`,
        `lesson=${r.lesson}`,
        `teacher=${r.teacher}`,
        r.rv00 ? `sheer_rv00=${r.rv00}` : ""
    ].filter(Boolean);

    return {
        summary,
        description: detailsLines.join("\n"),
        start: { dateTime: formatLocalDateTimeForGCal(start) },
        end: { dateTime: formatLocalDateTimeForGCal(end) }
    };
}

async function syncPage() {
    if (isSyncRunning) return;
    isSyncRunning = true;

    const bar = ensureTopBar();
    const btn = bar.querySelector("#sheer-to-gcal-sync");
    const prevText = btn.textContent;
    btn.disabled = true;
    btn.textContent = "Syncing...";

    const auth = await chrome.runtime.sendMessage({ type: "auth:status" });
    if (!auth?.authorized) {
        btn.disabled = false;
        btn.textContent = prevText;
        isSyncRunning = false;
        alert("Not connected. Open extension options and click Connect Google.");
        return;
    }

    try {
        const calendarId = await getCalendarId();
        const state = await getSyncState();
        const reservations = Array.from(document.querySelectorAll(".Reserve0Table tbody tr"))
            .map((tr) => buildReservationFromRow(tr))
            .filter(Boolean);

        // 1) Delete cancelled ones we previously synced.
        await reconcileCancelled(state, reservations);

        // 2) Self-heal if the eventId was manually deleted on Google Calendar.
        await reconcileMissingEvents(state, reservations);

        // 3) Create missing ones.
        for (const r of reservations) {
            if (!(r.state === "scheduled" || r.state === "done")) continue;

            const existingEventId = state.eventsByKey[r.key]?.eventId;
            const event = buildEventPayload(r);

            if (existingEventId) {
                // Apply latest payload to already-synced reservations to follow spec changes.
                const exists = await chrome.runtime.sendMessage({ type: "gcal:get", calendarId, eventId: existingEventId });
                if (exists?.exists === false) {
                    delete state.eventsByKey[r.key];
                } else {
                    // Only update events that look like they were created by this extension.
                    // This avoids overwriting manually-created/edited events.
                    const desc = String(exists?.description ?? "");
                    if (desc.includes("sheer_rv00=")) {
                        await chrome.runtime.sendMessage({ type: "gcal:update", calendarId, eventId: existingEventId, patch: event });
                    }
                    continue;
                }
            }

            // Mark in-flight to avoid double-click duplicates and multi-tab races.
            state.eventsByKey[r.key] = { status: "in_flight", startedAt: Date.now(), rv00: r.rv00 ?? null };
            await setSyncState(state);

            const res = await chrome.runtime.sendMessage({ type: "gcal:insert", calendarId, event });
            state.eventsByKey[r.key] = { eventId: res.eventId, createdAt: Date.now(), rv00: r.rv00 ?? null };
            await setSyncState(state);
        }

        await updateSummary();
        alert("Sync complete");
    } finally {
        btn.disabled = false;
        btn.textContent = prevText;
        isSyncRunning = false;
    }
}

function installRowBadges() {
    (async () => {
        const state = await getSyncState();
        const rows = Array.from(document.querySelectorAll(".Reserve0Table tbody tr"));
        for (const tr of rows) {
            const r = buildReservationFromRow(tr);
            if (!r) continue;
            const tds = tr.querySelectorAll("td");
            const stateTd = tds[4];
            const badge = document.createElement("span");
            badge.style.cssText = "margin-left:6px;font-size:12px;color:#555";
            const known = state.eventsByKey[r.key];
            if (r.state === "cancelled") {
                badge.textContent = known?.eventId ? "(will delete on sync)" : "";
            } else if (r.state === "scheduled" || r.state === "done") {
                badge.textContent = known?.eventId ? "(synced)" : "(not synced)";
            }
            stateTd.appendChild(badge);
        }
    })();
}

(async () => {
    const bar = ensureTopBar();
    bar.querySelector("#sheer-to-gcal-sync").addEventListener("click", syncPage);
    installRowBadges();
    await updateSummary();
})();
