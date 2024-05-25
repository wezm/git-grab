mod providers;

use std::io;

use self::providers::Clipboard;

pub fn provider() -> io::Result<Box<dyn Clipboard>> {
    providers::provide()
}
