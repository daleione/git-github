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

/// The animated loading text for a given `frame`: the title typed out one
/// character at a time, then a cycling 0–3 dot "working" suffix (it loops back
/// to none rather than growing without bound).
fn loading_label(frame: usize, title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    if frame < chars.len() {
        // Still typing the title out.
        chars[..=frame].iter().collect()
    } else {
        // Title fully shown; cycle the trailing dots (advancing every 2 frames).
        let dots = (frame - chars.len()) / 2 % 4;
        let mut label: String = chars.iter().collect();
        for _ in 0..dots {
            label.push('.');
        }
        label
    }
}

/// A loading spinner shown on the header line while the model generates. The
/// glyph spins on its own thread, animating `⠋ title`, until [`Spinner::stop`]
/// halts it — leaving the cursor at the start of that same line so the caller
/// can transform it in place (see [`reveal`]).
///
/// When styling is disabled no thread is spawned and nothing is printed, so
/// piped output and `NO_COLOR` stay clean.
pub struct Spinner {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Spinner {
    /// Start the animation, preceded by a blank line separating it from the
    /// previous output.
    pub fn start(title: &str) -> Spinner {
        if !enabled() {
            return Spinner {
                stop: Arc::new(AtomicBool::new(true)),
                handle: None,
            };
        }
        println!();

        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let title_thread = title.to_string();
        let handle = thread::spawn(move || {
            let mut frame = 0;
            while !stop_thread.load(Ordering::Relaxed) {
                let glyph = SPIN_FRAMES[frame % SPIN_FRAMES.len()];
                let label = loading_label(frame, &title_thread);
                // `2K` clears the line so shrinking dots leave no leftovers.
                print!("\r\x1b[2K{} {}", cyan(glyph), bold(&label));
                let _ = std::io::stdout().flush();
                frame += 1;
                thread::sleep(Duration::from_millis(80));
            }
        });

        Spinner {
            stop,
            handle: Some(handle),
        }
    }

    /// Halt the animation, leaving the cursor at column 0 of the header line.
    pub fn stop(mut self) {
        if let Some(handle) = self.handle.take() {
            self.stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
            print!("\r");
            let _ = std::io::stdout().flush();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        // Ensure the thread is signalled even if `stop` was never called.
        if let Some(handle) = self.handle.take() {
            self.stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
        }
    }
}

/// Transform the header line — assumed to be the cursor's current line — from
/// the static `✦ title` label into the generated `message`, all in place:
/// settle the label, retract it character by character, then type the message
/// out on the same line. Extra message lines (if any) follow under a gutter.
///
/// When styling is disabled the message is printed plainly under a gutter, so
/// piped output is unchanged.
pub fn reveal(title: &str, message: &str) {
    let message = message.trim_end();
    if !enabled() {
        for line in message.lines() {
            println!("{} {}", gutter(), line);
        }
        return;
    }

    // 1. Keep the glyph spinning and the dots cycling for a couple of cycles
    //    (the title is already fully typed by now), then settle into `✦` and
    //    hold a beat.
    let grown = title.chars().count();
    for step in 0..SPIN_FRAMES.len() * 2 {
        let glyph = SPIN_FRAMES[step % SPIN_FRAMES.len()];
        let label = loading_label(grown + step, title);
        print!("\r\x1b[2K{} {}", cyan(glyph), bold(&label));
        let _ = std::io::stdout().flush();
        thread::sleep(Duration::from_millis(60));
    }
    print!("\r\x1b[2K{} {}", cyan("✦"), bold(title));
    let _ = std::io::stdout().flush();
    thread::sleep(Duration::from_millis(180));

    // 2. Retract the label one character at a time, keeping the `✦ ` glyph.
    let label: Vec<char> = title.chars().collect();
    for keep in (0..label.len()).rev() {
        let shown: String = label[..keep].iter().collect();
        print!("\r\x1b[2K{} {}", cyan("✦"), dim(&shown));
        let _ = std::io::stdout().flush();
        thread::sleep(Duration::from_millis(16));
    }

    // 3. Type the message onto the same line, after the `✦ `. Any further
    //    lines stream below under the gutter.
    let mut lines = message.lines();
    if let Some(first) = lines.next() {
        print!("\r\x1b[2K{} ", cyan("✦"));
        type_out(first);
        println!();
    }
    for line in lines {
        print!("{} ", gutter());
        type_out(line);
        println!();
    }
    let _ = std::io::stdout().flush();
}

/// Clear the cursor's current line and return to its start. Used to wipe a
/// stopped spinner when there's nothing to reveal. No-op when disabled.
pub fn clear_line() {
    if enabled() {
        print!("\r\x1b[2K");
        let _ = std::io::stdout().flush();
    }
}

/// Print `text` one character at a time, flushing and pausing between each.
fn type_out(text: &str) {
    for ch in text.chars() {
        print!("{ch}");
        let _ = std::io::stdout().flush();
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
