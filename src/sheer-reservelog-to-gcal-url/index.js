function pad2(n) {
    return String(n).padStart(2, "0");
}

function addMinutes(date, minutes) {
    return new Date(date.getTime() + minutes * 60 * 1000);
}

function normalizeSpace(s) {
    return s.replace(/\s+/g, " ").trim();
}

function formatDateUtcForGCal(date) {
    // Google Calendar TEMPLATE expects UTC in YYYYMMDDTHHMMSSZ.
    // Convert from local Date to UTC fields.
    return (
        `${date.getUTCFullYear()}${pad2(date.getUTCMonth() + 1)}${pad2(date.getUTCDate())}` +
        `T${pad2(date.getUTCHours())}${pad2(date.getUTCMinutes())}00Z`
    );
}

function parseDateTimeFromCell(td) {
    const text = normalizeSpace(td.innerText);
    const m = text.match(/(\d{4}-\d{2}-\d{2})\s+(\d{2}:\d{2})/);
    if (!m) return null;
    const [_, d, t] = m;
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

function getStorageKey() {
    return "sheerToGcalUrl.state.v1";
}

function getState() {
    try {
        return JSON.parse(localStorage.getItem(getStorageKey()) ?? "null") ?? { addedByKey: {} };
    } catch {
        return { addedByKey: {} };
    }
}

function setState(state) {
    localStorage.setItem(getStorageKey(), JSON.stringify(state));
}

function buildTemplateUrl(r) {
    const durationMin = 45;
    const start = r.start;
    const end = addMinutes(start, durationMin);

    const text = `SHEER ${r.studio} ${r.lesson} ${r.teacher}`;
    const detailsLines = [
        `studio=${r.studio}`,
        `lesson=${r.lesson}`,
        `teacher=${r.teacher}`,
        r.rv00 ? `sheer_rv00=${r.rv00}` : ""
    ].filter(Boolean);

    const dates = `${formatDateUtcForGCal(start)}/${formatDateUtcForGCal(end)}`;
    const url = new URL("https://calendar.google.com/calendar/render");
    url.searchParams.set("action", "TEMPLATE");
    url.searchParams.set("text", text);
    url.searchParams.set("dates", dates);
    url.searchParams.set("details", detailsLines.join("\n"));
    return url.toString();
}

function ensureTopBar() {
    let bar = document.getElementById("sheer-to-gcal-url-bar");
    if (bar) return bar;
    bar = document.createElement("div");
    bar.id = "sheer-to-gcal-url-bar";
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
        <button id="sheer-to-gcal-url-add-all">Add all (new)</button>
        <span id="sheer-to-gcal-url-summary" style="color:#333"></span>
        <button id="sheer-to-gcal-url-reset">Reset flags</button>
    `;
    const main = document.querySelector("#Main") ?? document.body;
    main.prepend(bar);
    return bar;
}

function collectReservations() {
    return Array.from(document.querySelectorAll(".Reserve0Table tbody tr"))
        .map((tr) => ({ tr, r: buildReservationFromRow(tr) }))
        .filter(({ r }) => r);
}

function updateSummary() {
    const bar = ensureTopBar();
    const state = getState();
    const rows = collectReservations();
    const candidates = rows.filter(({ r }) => r.state === "scheduled" || r.state === "done");
    const newCount = candidates.filter(({ r }) => !state.addedByKey[r.key]).length;
    bar.querySelector("#sheer-to-gcal-url-summary").textContent = `New: ${newCount}`;
}

function installRowControls() {
    const state = getState();
    for (const { tr, r } of collectReservations()) {
        const tds = tr.querySelectorAll("td");
        const stateTd = tds[4];
        if (r.state !== "scheduled" && r.state !== "done") continue;

        const btn = document.createElement("button");
        btn.textContent = state.addedByKey[r.key] ? "Added" : "Add";
        btn.disabled = !!state.addedByKey[r.key];
        btn.style.cssText = "margin-left:8px";
        btn.addEventListener("click", () => {
            const url = buildTemplateUrl(r);
            window.open(url, "_blank", "noopener,noreferrer");
            const s = getState();
            s.addedByKey[r.key] = { openedAt: Date.now() };
            setState(s);
            btn.textContent = "Added";
            btn.disabled = true;
            updateSummary();
        });
        stateTd.appendChild(btn);
    }
}

function installTopBarHandlers() {
    const bar = ensureTopBar();
    bar.querySelector("#sheer-to-gcal-url-add-all").addEventListener("click", () => {
        const state = getState();
        const candidates = collectReservations()
            .map(({ r }) => r)
            .filter((r) => r && (r.state === "scheduled" || r.state === "done"));
        const newOnes = candidates.filter((r) => !state.addedByKey[r.key]);
        for (const r of newOnes) {
            const url = buildTemplateUrl(r);
            window.open(url, "_blank", "noopener,noreferrer");
            state.addedByKey[r.key] = { openedAt: Date.now() };
        }
        setState(state);
        location.reload();
    });

    bar.querySelector("#sheer-to-gcal-url-reset").addEventListener("click", () => {
        if (!confirm("Reset local added flags?")) return;
        setState({ addedByKey: {} });
        location.reload();
    });
}

(function main() {
    if (!location.href.startsWith("https://reservations-sheer.jp/user/reservelog.php")) return;
    ensureTopBar();
    installTopBarHandlers();
    installRowControls();
    updateSummary();
})();

