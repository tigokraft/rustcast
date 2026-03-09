const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let appContainer;
let globalOverlay;
let systemScreenOverlay;
let allSystemScreens;

// Function to pop a global notification overlay
export function showToast(message, durationMs = 3000) {
    const toast = document.createElement("div");
    toast.className = "toast";
    toast.textContent = message;
    
    globalOverlay.appendChild(toast);
    
    setTimeout(() => {
        toast.style.opacity = '0';
        toast.style.transform = 'translateY(-100px)';
        toast.style.transition = 'all 0.3s ease-in';
        setTimeout(() => toast.remove(), 300);
    }, durationMs);
}

// Keyboard input mapping to RemoteInput
const KEY_MAP = {
    "ArrowUp": "Up",
    "ArrowDown": "Down",
    "ArrowLeft": "Left",
    "ArrowRight": "Right",
    "Enter": "Select",
    "Escape": "Back",
    " ": "PlayPause",
    "+": "VolumeUp",
    "-": "VolumeDown",
};

window.addEventListener("DOMContentLoaded", async () => {
    appContainer = document.querySelector("#app-container");
    globalOverlay = document.querySelector("#global-overlay");

    // Attempt to load apps 
    try {
        const apps = await invoke("get_available_apps", {});
        showToast(`Loaded ${apps.length} apps via Wasm`);
    } catch(e) {
        showToast("Error loading apps.json");
    }

    // Subscribe to system-toast events from the D-Bus backend wrapper
    listen('system-toast', (event) => {
        showToast(event.payload.message || event.payload);
    });

    systemScreenOverlay = document.getElementById("system-screen");
    allSystemScreens = document.querySelectorAll(".screen-content");

    // System Screens Logic
    function showSystemScreen(screenId) {
        if (!screenId) {
            systemScreenOverlay.classList.add('hidden');
            allSystemScreens.forEach(s => s.classList.add('hidden'));
            return;
        }
        
        systemScreenOverlay.classList.remove('hidden');
        allSystemScreens.forEach(s => {
            if (s.id === `screen-${screenId}`) {
                s.classList.remove('hidden');
            } else {
                s.classList.add('hidden');
            }
        });
    }

    // Subscribe to system screen states (startup, shutdown, setup)
    listen('system-screen-update', (event) => {
        showSystemScreen(event.payload);
    });

    // Subscribe to dynamic hot-reloading theme configs
    listen('theme-update', (event) => {
        const theme = event.payload;
        const root = document.documentElement;
        if (theme.primary_color) root.style.setProperty('--vibe-primary', theme.primary_color);
        if (theme.accent_color) root.style.setProperty('--vibe-accent', theme.accent_color);
        if (theme.background_vibe) root.style.setProperty('--vibe-bg', theme.background_vibe);
        if (theme.border_radius) root.style.setProperty('--vibe-radius', theme.border_radius);
        if (theme.font_family) root.style.setProperty('--vibe-font', theme.font_family);
        showToast("Theme UI configuration reloaded.");
    });

    // Handle User Input mapping globally on the entire focus layer
    window.addEventListener("keydown", async (e) => {
        const action = KEY_MAP[e.key];
        if (action) {
            e.preventDefault();
            // Wrap in quote string format to match Serde enum variant representation
            const payload = JSON.stringify(action);
            const nextUiState = await invoke("send_remote_input", { input: payload });
            try { appContainer.innerHTML = `<pre>${JSON.stringify(JSON.parse(nextUiState), null, 2)}</pre>`; } catch { appContainer.innerHTML = `<pre>${nextUiState}</pre>`; }
        }
    });

    // Establish WebSocket Connection to local remote-server
    function connectRemoteServer() {
        const ws = new WebSocket("ws://127.0.0.1:8002/ws/remote");
        ws.onmessage = async (event) => {
            // The message from WS is already a serialized `RemoteInput` JSON string
            const nextUiState = await invoke("send_remote_input", { input: event.data });
            try { appContainer.innerHTML = `<pre>${JSON.stringify(JSON.parse(nextUiState), null, 2)}</pre>`; } catch { appContainer.innerHTML = `<pre>${nextUiState}</pre>`; }
        };
        ws.onerror = () => setTimeout(connectRemoteServer, 5000);
    }
    connectRemoteServer();

    // Request an initial render of whatever app was automatically loaded
    try {
        const activeName = await invoke("get_active_app", {});
        if (activeName) {
            const nextUiState = await invoke("send_remote_input", { input: "\"Select\"" }); // Mock input
            try { appContainer.innerHTML = `<pre>${JSON.stringify(JSON.parse(nextUiState), null, 2)}</pre>`; } catch { appContainer.innerHTML = `<pre>${nextUiState}</pre>`; }
        }
    } catch {}
});
