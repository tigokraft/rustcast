use anyhow::Result;
use core_proto::{AppMetadata, RemoteInput, VibeApp};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use wasmtime::{Engine, Instance, Linker, Module, Store};

#[derive(Debug, Deserialize)]
pub struct AppsManifest {
    pub apps: Vec<AppDefinition>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppDefinition {
    pub id: String,
    pub name: String,
    pub executable: String,
}

/// Wraps a WebAssembly instance to implement the VibeApp trait from the Host side.
pub struct WasmApp {
    pub id: String,
    pub name: String,
    instance: Instance,
    store: Arc<Mutex<Store<()>>>,
}

impl WasmApp {
    pub fn new(engine: &Engine, path: impl AsRef<Path>, id: String, name: String) -> Result<Self> {
        let mut store = Store::new(engine, ());
        let module = Module::from_file(engine, path)?;
        let linker = Linker::new(engine);

        // Here we could inject host functions (like logging) into the Wasm module via linker
        // linker.func_wrap("env", "host_log", |caller, ptr: i32, len: i32| { ... })?;

        let instance = linker.instantiate(&mut store, &module)?;

        Ok(Self {
            id,
            name,
            instance,
            store: Arc::new(Mutex::new(store)),
        })
    }
}

impl VibeApp for WasmApp {
    fn metadata(&self) -> AppMetadata {
        // In a real implementation, we would extract a pointer to memory containing JSON/bincode
        // and deserialize it into `AppMetadata`. For now, we return a mock.
        AppMetadata {
            name: "Wasm Plugin".to_string(),
            version: "1.0.0".to_string(),
            author: "Unknown".to_string(),
        }
    }

    fn on_init(&mut self) {
        let mut store = self.store.lock().unwrap();
        // Invoke the 'on_init' export if the plugin provides it
        if let Ok(func) = self.instance.get_typed_func::<(), ()>(&mut store, "on_init") {
            let _ = func.call(&mut store, ());
        }
    }

    fn handle_input(&mut self, _input: RemoteInput) {
        let mut store = self.store.lock().unwrap();
        // Here we would serialize `RemoteInput` and pass the pointer to the Wasm instance.
        // E.g., func.call(&mut store, (input_enum_value))
        if let Ok(func) = self
            .instance
            .get_typed_func::<i32, ()>(&mut store, "handle_input")
        {
            let _ = func.call(&mut store, 0); // 0 is a stub for serialized input
        }
    }

    fn render(&self) -> String {
        let mut store = self.store.lock().unwrap();
        // We'd call logic that returns a memory pointer + length containing the UI State Tree or HTML
        if let Ok(func) = self.instance.get_typed_func::<(), ()>(&mut store, "render") {
            let _ = func.call(&mut store, ());
        }
        "{\"type\": \"container\", \"children\": []}".to_string()
    }

    fn on_shutdown(&mut self) {
        let mut store = self.store.lock().unwrap();
        if let Ok(func) = self
            .instance
            .get_typed_func::<(), ()>(&mut store, "on_shutdown")
        {
            let _ = func.call(&mut store, ());
        }
    }
}

/// The Registry tracks all loaded Wasm apps and manages the active state
pub struct Registry {
    engine: Engine,
    pub apps: HashMap<String, WasmApp>,
    pub active_app: Option<String>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            engine: Engine::default(),
            apps: HashMap::new(),
            active_app: None,
        }
    }

    /// Scans a directory for apps.json and attempts to instantiate the listed apps
    pub fn scan_and_load(&mut self, apps_dir: impl AsRef<Path>) -> Result<()> {
        let dir = apps_dir.as_ref();
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }

        let manifest_path = dir.join("apps.json");
        if !manifest_path.exists() {
            eprintln!("Warning: apps.json not found in {:?}", dir);
            return Ok(());
        }

        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: AppsManifest = serde_json::from_str(&manifest_content)?;

        for app_def in manifest.apps {
            let wasm_path = dir.join(&app_def.executable);
            if wasm_path.exists() {
                match WasmApp::new(&self.engine, &wasm_path, app_def.id.clone(), app_def.name.clone()) {
                    Ok(app) => {
                        println!("Successfully loaded WasmApp: {}", app_def.name);
                        self.apps.insert(app_def.id.clone(), app);
                        
                        // Default focus to the first loaded app if none active
                        if self.active_app.is_none() {
                            self.active_app = Some(app_def.id);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to load WasmApp from {:?}: {}", wasm_path, e);
                    }
                }
            } else {
                eprintln!("Wasm file not found for app {}: {:?}", app_def.id, wasm_path);
            }
        }
        Ok(())
    }

    /// Sets the active app by its registry key (filename stem)
    pub fn set_active(&mut self, name: &str) {
        if self.apps.contains_key(name) {
            self.active_app = Some(name.to_string());
        }
    }

    /// Returns a mutable reference to the currently active app
    pub fn active_app_mut(&mut self) -> Option<&mut WasmApp> {
        if let Some(name) = &self.active_app {
            self.apps.get_mut(name)
        } else {
            None
        }
    }

    /// Returns a reference to the currently active app
    pub fn active_app(&self) -> Option<&WasmApp> {
        if let Some(name) = &self.active_app {
            self.apps.get(name)
        } else {
            None
        }
    }
}
