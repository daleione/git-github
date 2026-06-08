//! Tiny, dependency-free terminal styling. Color and decorative glyphs are
//! emitted only when stdout is a real terminal and `NO_COLOR` is unset, so
//! piped output stays plain and script-friendly.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Whether to emit ANSI color and glyphs. Cached on first use.
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
    })
}

const RESET: &str = "\x1b[0m";

fn paint(code: &str, s: &str) -> String {
    if enabled() {
        format!("\x1b[{code}m{s}{RESET}")
    } else {
        s.to_string()
    }
}

pub fn bold(s: &str) -> String {
    paint("1", s)
}
pub fn dim(s: &str) -> String {
    paint("2", s)
}
pub fn green(s: &str) -> String {
    paint("32", s)
}
pub fn cyan(s: &str) -> String {
    paint("36", s)
}

/// A section header introducing AI/streamed output, e.g.
/// "Suggested commit message". Renders as `✦ title` when styled, else plain.
/// A blank line above separates it from whatever came before.
pub fn header(title: &str) {
    println!();
    if enabled() {
        println!("{} {}", cyan("✦"), bold(title));
    } else {
        println!("{title}");
    }
}

/// A success line, e.g. "Committed 94d0aca on main". Renders as `✓ msg` in
/// green when styled, else just `msg`. A blank line above sets it apart from
/// the preceding output (the message body, prompt, or git's own chatter).
pub fn success(msg: &str) {
    println!();
    if enabled() {
        println!("{} {}", green("✓"), msg);
    } else {
        println!("{msg}");
    }
}

/// Format an interactive question: `? question  hints ` — the `?` cyan, the
/// question bold, the hints dimmed. Plain `question hints ` when unstyled.
pub fn prompt(question: &str, hints: &str) -> String {
    if enabled() {
        format!("{} {}  {} ", cyan("?"), bold(question), dim(hints))
    } else {
        format!("{question} {hints} ")
    }
}

/// Move up `n` lines and clear from there to the end of the screen, making a
/// transient prompt (and its leading blank) vanish once it has been answered.
/// No-op when styling is disabled, so piped output keeps the prompt intact.
pub fn erase_lines(n: usize) {
    if enabled() && n > 0 {
        print!("\x1b[{n}A\x1b[0J");
        let _ = std::io::stdout().flush();
    }
}

/// Braille frames for the loading spinner (same family Claude Code uses).
const SPIN_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// An animated header shown while the model is generating. The leading glyph
/// spins on its own thread until [`Spinner::finish`] settles it into the static
/// `✦ title`, after which the streamed body follows on the next line.
///
/// When styling is disabled the header is printed once, statically, and no
/// thread is spawned — so piped output and `NO_COLOR` stay clean.
pub struct Spinner {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    title: String,
}

impl Spinner {
    /// Start the animation (preceded by a blank line, matching [`header`]).
    pub fn start(title: &str) -> Spinner {
        println!();
        if !enabled() {
            println!("{title}");
            return Spinner {
                stop: Arc::new(AtomicBool::new(true)),
                handle: None,
                title: title.to_string(),
            };
        }

        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let title_thread = title.to_string();
        let handle = thread::spawn(move || {
            let mut frame = 0;
            while !stop_thread.load(Ordering::Relaxed) {
                let glyph = SPIN_FRAMES[frame % SPIN_FRAMES.len()];
                print!("\r{} {}", cyan(glyph), bold(&title_thread));
                let _ = std::io::stdout().flush();
                frame += 1;
                thread::sleep(Duration::from_millis(80));
            }
        });

        Spinner {
            stop,
            handle: Some(handle),
            title: title.to_string(),
        }
    }

    /// Stop the animation and settle the header into a static `✦ title` on its
    /// own line, leaving the cursor ready for the streamed body below.
    pub fn finish(mut self) {
        if let Some(handle) = self.handle.take() {
            self.stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
            // Clear the spinner frame and write the final header.
            print!("\r\x1b[2K{} {}", cyan("✦"), bold(&self.title));
            println!();
            let _ = std::io::stdout().flush();
        }
        // When disabled the header was already printed in `start`.
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        // Ensure the thread is signalled even if `finish` was never called.
        if let Some(handle) = self.handle.take() {
            self.stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
        }
    }
}

/// Pace streamed output into a one-character-at-a-time "typewriter" reveal:
/// a short sleep between characters. No-op when styling is disabled, so piped
/// output streams at full speed.
pub fn tick() {
    if enabled() {
        thread::sleep(Duration::from_millis(8));
    }
}

/// The gutter prefix for each streamed body line. A dim `│` when styled, a
/// plain `|` otherwise (unchanged from the original piped behavior).
pub fn gutter() -> String {
    if enabled() {
        dim("│")
    } else {
        "|".to_string()
    }
}
