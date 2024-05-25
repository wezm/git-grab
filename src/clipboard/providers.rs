//! implements different clipboard types

use std::{
    io::{self, Read, Write},
    process::{Command, Stdio},
};

pub trait Clipboard {
    fn copy(&self, text: &str) -> io::Result<()>;
    fn paste(&self) -> io::Result<String>;
}

macro_rules! c {
    ($p:ident $($args:ident)+) => {
        Command::new(stringify!($p)).args([$(stringify!($args),)+])
    };
    ($p:literal) => {
        Command::new($p)
    };
    ($p:literal $($args:literal)+) => {
        Command::new($p).args([$($args,)+])

    }
}

trait Eat {
    fn eat(&mut self) -> io::Result<String>;
}

impl Eat for Command {
    fn eat(&mut self) -> io::Result<String> {
        let mut s = String::new();
        self.stdout(Stdio::piped())
            .spawn()?
            .stdout
            .take()
            .expect("stdout")
            .read_to_string(&mut s)?;
        Ok(s)
    }
}

trait Put {
    fn put(&mut self, s: impl AsRef<[u8]>) -> io::Result<()>;
}

impl Put for Command {
    fn put(&mut self, s: impl AsRef<[u8]>) -> io::Result<()> {
        let mut ch = self.stdin(Stdio::piped()).spawn()?;
        ch.stdin.take().expect("stdin").write_all(s.as_ref())?;
        ch.wait()?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub struct PbCopy {}
#[cfg(target_os = "macos")]
impl Clipboard for PbCopy {
    fn copy(&self, text: &str) -> io::Result<()> {
        c!(pbcopy w).put(text)
    }

    fn paste(&self) -> io::Result<String> {
        c!(pbcopy r).eat()
    }
}

pub struct XClip {}
impl Clipboard for XClip {
    fn copy(&self, text: &str) -> io::Result<()> {
        c!("xclip" "-selection" "c").put(text)
    }

    fn paste(&self) -> io::Result<String> {
        c!("xclip" "-selection" "c" "-o") // xclip is complainy
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .eat() // If stdout is nulled does this work?
    }
}

pub struct XSel {}
impl Clipboard for XSel {
    fn copy(&self, text: &str) -> io::Result<()> {
        c!("xsel" "-b" "-i").put(text)
    }

    fn paste(&self) -> io::Result<String> {
        c!("xsel" "-b" "-o").eat()
    }
}

struct Wayland {}
impl Clipboard for Wayland {
    fn copy(&self, text: &str) -> io::Result<()> {
        match text {
            "" => c!("wl-copy" "-p" "--clear")
                .status()?
                .success()
                .then_some(())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        String::from("wl-copy was not successful"),
                    )
                }),
            s => c!("wl-copy" "-p").put(s),
        }
    }

    fn paste(&self) -> io::Result<String> {
        c!("wl-paste" "-n" "-p").eat()
    }
}

struct Klipper {}
impl Clipboard for Klipper {
    fn copy(&self, text: &str) -> io::Result<()> {
        c!("qdbus" "org.kde.klipper" "/klipper" "setClipboardContents").arg(text);
        Ok(())
    }

    fn paste(&self) -> io::Result<String> {
        let mut s = c!("qdbus" "org.kde.klipper" "/klipper" "getClipboardContents").eat()?;
        assert!(s.ends_with('\n'));
        s.truncate(s.len() - 1);
        Ok(s)
    }
}

#[cfg(target_family = "windows")]
struct Windows {}
#[cfg(target_family = "windows")]
impl Clipboard for Windows {
    fn copy(&self, text: &str) -> io::Result<()> {
        clipboard_win::set_clipboard_string(text)?
    }

    fn paste(&self) -> io::Result<String> {
        clipboard_win::get_clipboard_string()?
    }
}

struct Wsl {}
impl Clipboard for Wsl {
    fn copy(&self, text: &str) -> io::Result<()> {
        c!("clip.exe").put(text)
    }

    fn paste(&self) -> io::Result<String> {
        let mut s = c!("powershell.exe" "-noprofile" "-command" "Get-Clipboard").eat()?;
        s.truncate(s.len() - 2); // \r\n
        Ok(s)
    }
}

fn has(c: &str) -> bool {
    c!("which")
        .arg(c)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .map_or(false, |status| status.success())
}

fn wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map_or(false, |s| s.to_lowercase().contains("microsoft"))
}

pub fn provide() -> io::Result<Box<dyn Clipboard>> {
    #[cfg(target_family = "windows")]
    return get::<Windows>();
    #[cfg(target_os = "macos")]
    return get::<PbCopy>();

    if wsl() {
        return Ok(Box::new(Wsl {}));
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && has("wl-copy") {
        Ok(Box::new(Wayland {}))
    } else if has("xsel") {
        Ok(Box::new(XSel {}))
    } else if has("xclip") {
        Ok(Box::new(XClip {}))
    } else if has("klipper") && has("qdbus") {
        Ok(Box::new(Klipper {}))
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            String::from("no clipboard provided available"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_provider(provider: impl Clipboard) {
        provider.copy("text").expect("unable to copy");
        assert_eq!(provider.paste().expect("unable to paste"), "text");
        provider.copy("").expect("unable to clear clipboard");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn pbcopy() {
        test_provider(PbCopy {});
    }

    #[test]
    #[cfg(all(unix, not(target_os = "macos")))]
    fn xclip() {
        if has("xclip") {
            test_provider(XClip {});
        }
    }

    #[test]
    #[cfg(all(unix, not(target_os = "macos")))]
    fn xsel() {
        if has("xsel") {
            test_provider(XSel {});
        }
    }

    #[test]
    #[cfg(all(unix, not(target_os = "macos")))]
    fn wayland() {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            test_provider(Wayland {});
        }
    }

    #[test]
    #[cfg(all(unix, not(target_os = "macos")))]
    fn klipper() {
        if has("klipper") {
            test_provider(Klipper {});
        }
    }

    #[test]
    #[cfg(windows)]
    fn windows() {
        test_provider(Windows {});
    }

    #[test]
    #[cfg(windows)]
    fn test_wsl() {
        if wsl() {
            test_provider(Wsl {});
        }
    }
}
