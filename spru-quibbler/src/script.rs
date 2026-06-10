cfg_select! {
    debug_assertions => {
        
    }
    _ => {
        use std::sync;
    }
}



/// Takes a string literal path to a bundled script file. 
/// In debug, the script will be hot-loaded each time it is accessed, in release, it
/// is cached after the first read.
macro_rules! script {
    ($path:literal) => {
        {
            // No boxed statics, no type infered statics, no named closures, so we are left with this...
            fn __load_script() -> String {
                cfg_select! {
                    all(target_family = "wasm", target_os = "unknown") => {
                        // Scripts needs to be packaged into the binary on WASM
                        include_str!(concat!(
                            env!("CARGO_MANIFEST_DIR"), 
                            "/assets/",
                            $path,
                        )).to_string()
                    }
                    _ => {
                        $crate::script::Script::read($path)
                    }
                }
            }
            
            $crate::script::Script::new(__load_script)
        }
    };
}

pub(crate) use script;

cfg_select! {
    debug_assertions => {
        pub struct Script {
            load: fn() -> String,
        }
    }
    _ => {
        pub struct Script {
            lazy: sync::LazyLock<String>
        }
    }
}

impl Script {
    pub const fn new(load: fn() -> String) -> Self {
        cfg_select! {
            debug_assertions => {
                Script { load }
            }
            _ => {
                Script {
                    lazy: sync::LazyLock::new(load),
                }
            }
        }
    }

    #[doc(hidden)]
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    pub(crate) fn read(path: &str) -> String {
        if let Ok(mut full_path) = std::env::current_exe() {
            full_path.pop();
            full_path.push(path);
            println!("{}", full_path.display());
            std::fs::read_to_string(full_path)
                .map_err(|e| panic!("Could not read script '{path}': {e}"))
                .unwrap()
        } else {
            unimplemented!()
        }
    }

    pub fn get(&self) -> String {
        cfg_select! {
            debug_assertions => {
                (self.load)()
            }
            _ => {
                self.lazy.clone()
            }
        }
    }
}
