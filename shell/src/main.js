const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let appContainer;
let globalOverlay;

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
        const remoteInput = KEY_MAP[e.key];
        if (remoteInput) {
            e.preventDefault();
            // Send focus input straight to rust backend, replacing UI locally
            const nextUiState = await invoke("send_remote_input", { input: remoteInput });
            // Since we use a mock state tree of `{"type": "container", "children": []}` from the Rust wrapper,
            // we will pretty print the JSON into the container
            try {
                const parsed = JSON.parse(nextUiState);
                appContainer.innerHTML = `<pre>${JSON.stringify(parsed, null, 2)}</pre>`;
            } catch {
                appContainer.innerHTML = `<pre>${nextUiState}</pre>`;
            }
        }
    });

    // Request an initial render of whatever app was automatically loaded
    try {
        const activeName = await invoke("get_active_app", {});
        if (activeName) {
            // Send an empty input just to yield the render state for now, 
            // since we don't have a direct `render_active()` exposed easily yet.
            const uiState = await invoke("send_remote_input", { input: "None" });
            const parsed = JSON.parse(uiState);
            appContainer.innerHTML = `<pre>${JSON.stringify(parsed, null, 2)}</pre>`;
        }
    } catch {}
});
