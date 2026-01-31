const $ = (sel) => document.querySelector(sel);

async function send(type, payload = {}) {
    try {
        const res = await chrome.runtime.sendMessage({ type, ...payload });
        if (res?.ok === false) {
            throw new Error(res.error ?? "Unknown error");
        }
        return res;
    } catch (e) {
        alert(`Background error: ${String(e?.message ?? e)}`);
        throw e;
    }
}

async function refreshStatus() {
    const res = await send("auth:status");
    $("#status").textContent = res?.authorized ? "Connected" : "Not connected";
}

$("#connect").addEventListener("click", async () => {
    await send("auth:connect");
    await refreshStatus();
});

$("#disconnect").addEventListener("click", async () => {
    await send("auth:disconnect");
    await refreshStatus();
});

$("#export").addEventListener("click", async () => {
    const res = await send("state:export");
    $("#state").value = JSON.stringify(res.state, null, 2);
});

$("#import").addEventListener("click", async () => {
    const txt = $("#state").value;
    const state = JSON.parse(txt);
    await send("state:import", { state });
    await refreshStatus();
});

$("#reset").addEventListener("click", async () => {
    await send("state:reset");
    $("#state").value = "";
});

refreshStatus();
