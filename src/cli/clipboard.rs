use std::process::Command;

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;

/// Get clipboard contents
#[allow(dead_code)]
pub fn read() -> Option<String> {
    if wsl::is_wsl() {
        // Run powershell through cmd.exe to not reset terminal settings
        let p = Command::new("cmd.exe")
            .args(&["/C", "powershell.exe", "Get-Clipboard"])
            .output()
            .expect("Failed to execute powershell Get-Clipboard");
        String::from_utf8(p.stdout).ok()
    } else {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        ctx.get_contents().ok()
    }
}

/// Quote and apply character escapes for powershell
fn powershell_quote(s: &str) -> String {
    format!("'{}'", s.replace("'", "''"))
}

/// Set clipboard contents
pub fn write(s: &str) {
    if wsl::is_wsl() {
        // Run powershell through cmd.exe to not reset terminal settings
        Command::new("cmd.exe")
            .args(&["/C", "powershell.exe", "Set-Clipboard", &powershell_quote(s)])
            .output()
            .expect("Failed to execute powershell Set-Clipboard");
    } else {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        ctx.set_contents(s.to_owned()).unwrap();
    }
}
