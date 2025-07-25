//! implements different clipboard types

use std::io;
#[cfg(unix)]
use std::{process::{Command, Stdio}, io::{Read, Write}};

pub trait Clipboard {
    #[allow(dead_code)]
    fn copy(&self, text: &str) -> io::Result<()>;
    fn paste(&self) -> io::Result<String>;
}

#[cfg(unix)]
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

#[cfg(unix)]
trait Eat {
    fn eat(&mut self) -> io::Result<String>;
}
#[cfg(unix)]
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

#[cfg(unix)]
trait Put {
    fn put(&mut self, s: impl AsRef<[u8]>) -> io::Result<()>;
}
#[cfg(unix)]
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
        c!("pbcopy").put(text)
    }

    fn paste(&self) -> io::Result<String> {
        c!("pbpaste" "-Prefer" "txt").eat()
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
pub struct XClip {}
#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
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

#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
pub struct XSel {}
#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
impl Clipboard for XSel {
    fn copy(&self, text: &str) -> io::Result<()> {
        c!("xsel" "-b" "-i").put(text)
    }

    fn paste(&self) -> io::Result<String> {
        c!("xsel" "-b" "-o").eat()
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
struct Wayland {}
#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
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


#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
struct Klipper {}
#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
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

#[cfg(windows)]
struct Windows {}
#[cfg(windows)]
impl Clipboard for Windows {
    fn copy(&self, text: &str) -> io::Result<()> {
        clipboard_win::set_clipboard_string(text)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))
    }

    fn paste(&self) -> io::Result<String> {
        clipboard_win::get_clipboard_string()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))
    }
}

#[cfg(target_os = "linux")]
struct Wsl {}
#[cfg(target_os = "linux")]
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

#[cfg(not(any(windows, target_os = "macos")))]
fn has(c: &str) -> bool {
    c!("which")
        .arg(c)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .map_or(false, |status| status.success())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map_or(false, |s| s.to_lowercase().contains("microsoft"))
}

#[cfg(windows)]
pub fn provide() -> io::Result<Box<dyn Clipboard>> {
    return Ok(Box::new(Windows {}));
}

#[cfg(target_os = "macos")]
pub fn provide() -> io::Result<Box<dyn Clipboard>> {
    return Ok(Box::new(PbCopy {}));
}

#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
pub fn provide() -> io::Result<Box<dyn Clipboard>> {
    if wsl() {
        Ok(Box::new(Wsl {}))
    } else if std::env::var_os("WAYLAND_DISPLAY").is_some() {
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
    #[cfg(not(any(windows, target_os = "macos")))]
    fn xclip() {
        if has("xclip") {
            test_provider(XClip {});
        }
    }

    #[test]
    #[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
    fn xsel() {
        if has("xsel") {
            test_provider(XSel {});
        }
    }

    #[test]
    #[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
    fn wayland() {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            test_provider(Wayland {});
        }
    }

    #[test]
    #[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "android")))]
    fn klipper() {
        if has("klipper") {
            test_provider(Klipper {});
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_wsl() {
        if wsl() {
            test_provider(Wsl {});
        }
    }

    #[test]
    #[cfg(windows)]
    fn windows() {
        test_provider(Windows {});
    }
}
