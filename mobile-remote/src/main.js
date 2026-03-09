const { listen } = window.__TAURI__.event;

let ws = null;
let statusEl = document.getElementById("connection-status");

// Handle UI Tab Switching
document.querySelectorAll(".tab-btn").forEach(btn => {
    btn.addEventListener("click", (e) => {
        document.querySelectorAll(".tab-btn").forEach(b => b.classList.remove("active"));
        document.querySelectorAll(".view").forEach(v => v.classList.remove("active"));
        e.target.classList.add("active");
        
        let targetViewId = e.target.id === "tab-dpad" ? "view-dpad" : "view-search";
        document.getElementById(targetViewId).classList.add("active");
    });
});

// Broadcast dispatcher
function sendEvent(payload) {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(payload));
    }
}

// Map taps on `.dpad-btn` and `.control-btn` dynamically into string properties for JSON structs
const interactiveButtons = [...document.querySelectorAll(".dpad-btn"), ...document.querySelectorAll(".control-btn")];
interactiveButtons.forEach(btn => {
    // We bind to `touchstart` if available for instant mobile feedback, fallback to click
    const action = btn.dataset.action;
    btn.addEventListener("pointerdown", (e) => {
        e.preventDefault(); // Prevents double firing click events on touch
        sendEvent(action);  // e.g. "Up", "Left", "PlayPause"
    });
});

// Map Keyboard Typing into Search string
const searchInput = document.getElementById("search-input");
searchInput.addEventListener("input", (e) => {
    sendEvent({ Search: e.target.value });
});

// Setup mDNS Discovery Hook
async function initNetworking() {
    listen('tv-discovered', (event) => {
        const tvUrl = event.payload; // e.g., "192.168.1.50:8002"
        statusEl.textContent = `Connecting to ${tvUrl}...`;

        if (ws) {
            ws.close();
        }

        ws = new WebSocket(`ws://${tvUrl}/ws/remote`);

        ws.onopen = () => {
            statusEl.textContent = "Connected to TV";
            statusEl.className = "status-connected";
        };

        ws.onclose = () => {
            statusEl.textContent = "Disconnected - Scanning...";
            statusEl.className = "status-disconnected";
            ws = null;
        };
        
        ws.onerror = () => {
             // Fallback to offline on failure
             statusEl.textContent = "Connection Error";
             statusEl.className = "status-disconnected";
        }
    });
}

// On mount
window.addEventListener("DOMContentLoaded", () => {
   initNetworking();
});
