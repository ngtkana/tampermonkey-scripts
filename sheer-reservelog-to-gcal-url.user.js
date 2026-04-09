// ==UserScript==
// @name         SHEER reservlog -> Google Calendar (URL)
// @namespace    https://ngtkana.local/
// @version      1.0.0
// @description  Add buttons to SHEER reservlog and open Google Calendar TEMPLATE URLs (no OAuth)
// @match        https://reservations-sheer.jp/user/reservelog.php*
// @grant        none
// @downloadURL  https://raw.githubusercontent.com/ngtkana/tampermonkey-scripts/main/dist/sheer-reservelog-to-gcal-url.user.js
// @updateURL    https://raw.githubusercontent.com/ngtkana/tampermonkey-scripts/main/dist/sheer-reservelog-to-gcal-url.user.js
// ==/UserScript==

(() => {
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __commonJS = (cb, mod) => function __require() {
    return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
  };

  // src/sheer-reservelog-to-gcal-url/index.js
  var require_index = __commonJS({
    "src/sheer-reservelog-to-gcal-url/index.js"() {
      function pad2(n) {
        return String(n).padStart(2, "0");
      }
      function addMinutes(date, minutes) {
        return new Date(date.getTime() + minutes * 60 * 1e3);
      }
      function normalizeSpace(s) {
        return s.replace(/\s+/g, " ").trim();
      }
      function getMainText(td) {
        const clone = td.cloneNode(true);
        clone.querySelectorAll("span").forEach((s) => s.remove());
        return normalizeSpace(clone.innerText);
      }
      function formatDateUtcForGCal(date) {
        return `${date.getUTCFullYear()}${pad2(date.getUTCMonth() + 1)}${pad2(date.getUTCDate())}T${pad2(date.getUTCHours())}${pad2(date.getUTCMinutes())}00Z`;
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
        if (text.startsWith("\u4E88\u7D04\u5B8C\u4E86")) return "scheduled";
        if (text.startsWith("\u30EC\u30C3\u30B9\u30F3\u6E08")) return "done";
        if (text.startsWith("\u30AD\u30E3\u30F3\u30BB\u30EB")) return "cancelled";
        return "unknown";
      }
      function buildReservationFromRow(tr) {
        const tds = tr.querySelectorAll("td");
        if (tds.length < 5) return null;
        const start = parseDateTimeFromCell(tds[0]);
        if (!start) return null;
        const studio = normalizeSpace(tds[1].innerText);
        const lesson = getMainText(tds[2]);
        const teacher = normalizeSpace(tds[3].innerText);
        const state = getStateLabel(tds[4]);
        const rv00 = extractRv00FromTd(tds[4]);
        return { rv00, start, studio, lesson, teacher, state };
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
      function collectReservations() {
        return Array.from(document.querySelectorAll(".Reserve0Table tbody tr")).map((tr) => ({ tr, r: buildReservationFromRow(tr) })).filter(({ r }) => r);
      }
      function installRowControls() {
        for (const { tr, r } of collectReservations()) {
          const tds = tr.querySelectorAll("td");
          const stateTd = tds[4];
          if (r.state !== "scheduled" && r.state !== "done") continue;
          const btn = document.createElement("button");
          btn.textContent = "Add";
          btn.style.cssText = "margin-left:8px";
          btn.addEventListener("click", () => {
            const url = buildTemplateUrl(r);
            window.open(url, "_blank", "noopener,noreferrer");
          });
          stateTd.appendChild(btn);
        }
      }
      (function main() {
        if (!location.href.startsWith("https://reservations-sheer.jp/user/reservelog.php")) return;
        installRowControls();
      })();
    }
  });
  require_index();
})();
