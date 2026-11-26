use std::io;

const APP_NAME: &str = "BlurAutoClicker";

#[cfg(target_os = "windows")]
const RUN_KEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";

#[cfg(not(target_os = "windows"))]
fn desktop_entry_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|dir| dir.join("autostart").join("blur-autoclicker.desktop"))
}

#[cfg(not(target_os = "windows"))]
fn desktop_entry_contents(enabled: bool) -> String {
    let exec = format!(
        "\"{}\" --autostart",
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| String::from("BlurAutoClicker"))
    );
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=BlurAutoClicker\n\
         Comment=Accuracy and performance focused auto clicker\n\
         Exec={exec}\n\
         Terminal=false\n\
         Hidden={}\n\
         X-GNOME-Autostart-enabled={}\n",
        if enabled { "false" } else { "true" },
        if enabled { "true" } else { "false" },
    )
}

pub fn get_autostart_enabled() -> bool {
    if crate::portable::is_portable() {
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(run_key) = hkcu.open_subkey(RUN_KEY) else {
            return false;
        };
        run_key.get_value::<String, _>(APP_NAME).is_ok()
    }

    #[cfg(not(target_os = "windows"))]
    {
        let Some(path) = desktop_entry_path() else {
            return false;
        };
        std::fs::read_to_string(path)
            .map(|contents| {
                contents.contains("X-GNOME-Autostart-enabled=true")
                    && !contents.contains("Hidden=true")
            })
            .unwrap_or(false)
    }
}

pub fn set_autostart_enabled(enabled: bool) -> io::Result<()> {
    if crate::portable::is_portable() {
        log::info!("[Autostart] Skipping autostart entry in portable mode");
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_WRITE)?;

        if enabled {
            let exe_path = std::env::current_exe()?;
            let value = format!("\"{}\" --autostart", exe_path.display());
            run_key.set_value(APP_NAME, &value)?;
        } else {
            let _ = run_key.delete_value(APP_NAME);
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let Some(path) = desktop_entry_path() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine the XDG config directory",
            ));
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, desktop_entry_contents(enabled))
    }
}
