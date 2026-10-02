"use strict";
const $ = (id) => document.getElementById(id);

const MISSING = "-";

const state = $("state");
const approval = $("approval");
const machineName = $("machine-name");
const tailscaleIpv4 = $("tailscale-ipv4");
const tailscaleIpv6 = $("tailscale-ipv6");
const fullDomain = $("full-domain");
const tags = $("tags");
const lastChecked = $("last-checked");

const preferencesForm = $("preferences-form");
const authKey = $("auth-key");
const requestedHostname = $("requested-hostname");
const requestedTags = $("requested-tags");
const ephemeral = $("ephemeral");
const controlServerUrl = $("control-server-url");
const preferencesMessage = $("preferences-message");

async function getJson(path) {
    const r = await fetch(path, {cache: "no-store"});
    if (!r.ok) throw new Error("HTTP " + r.status);
    return r.json();
}

async function postJson(path, body) {
    const r = await fetch(path, {
        method: "POST",
        headers: {"Content-Type": "application/json"},
        body: JSON.stringify(body),
    });
    if (!r.ok) throw new Error("HTTP " + r.status);
}

const STATES = {
    connecting: "Contacting the control server...",
    checking_authorization: "Checking whether this device is approved...",
    approved: "Approved; joining the tailnet...",
    awaiting_approval: "Waiting to be approved",
    opening_ports: "On the tailnet; opening ports...",
    reachable: "Serving",
    shutting_down: "Shutting down...",
    exiting: "Stopping after an error; restarting...",
};

function approvalLink(url) {
    const link = document.createElement("a");
    link.href = url;
    link.rel = "noreferrer";
    link.target = "_blank";
    link.textContent = url;
    return link;
}

function setStatusDetails(status) {
    state.textContent = (status && STATES[status.state]) || MISSING;
    approval.replaceChildren((status && status.approve_at) ? approvalLink(status.approve_at) : MISSING);
    machineName.textContent = (status && status.machine_name) || MISSING;
    tailscaleIpv4.textContent = (status && status.ipv4) || MISSING;
    tailscaleIpv6.textContent = (status && status.ipv6) || MISSING;
    fullDomain.textContent = (status && status.full_domain) || MISSING;
    tags.textContent = ((status && status.tags) || []).join(", ") || MISSING;
    lastChecked.textContent = new Date().toLocaleTimeString();
}

function setPreferences(preferences) {
    controlServerUrl.value = preferences.control_server_url;
    requestedHostname.value = preferences.requested_hostname || "";
    requestedTags.value = (preferences.requested_tags || []).join(", ");
    ephemeral.checked = preferences.ephemeral === true;
}

function setPreferencesDisabled(disabled) {
    for (const field of preferencesForm.elements) field.disabled = disabled;
}

async function pollStatus() {
    try {
        setStatusDetails(await getJson("api/status"));
    } catch (e) {
        console.warn("Could not read the status:", e);
        setStatusDetails(null);
    }
    // TODO: Implement long polling or pushing of status
    setTimeout(pollStatus, 5000);
}

async function init() {
    try {
        setPreferences(await getJson("api/preferences"));
        setPreferencesDisabled(false);
        preferencesMessage.textContent = "";
    } catch (e) {
        console.error("Could not read the preferences:", e);
        preferencesMessage.textContent = "Could not fetch the current preferences; reload the page.";
    }
    pollStatus();
}

preferencesForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    setPreferencesDisabled(true);
    try {
        preferencesMessage.textContent = "Saving...";
        await postJson("api/reconnect", {
            auth_key: authKey.value,
            control_server_url: controlServerUrl.value,
            requested_hostname: requestedHostname.value,
            requested_tags: requestedTags.value.split(",").map((t) => t.trim()).filter(Boolean),
            ephemeral: ephemeral.checked,
        });
        authKey.value = "";
        preferencesMessage.textContent = "Saved";
    } catch (e) {
        console.error("Could not reconnect:", e);
        preferencesMessage.textContent = "Could not save";
    }
    setPreferencesDisabled(false);
});

preferencesForm.addEventListener("invalid", (e) => {
    const section = e.target.closest("details");
    if (section) section.open = true;
}, true);

init();
