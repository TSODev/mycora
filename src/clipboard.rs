use std::io::{self, Write};

/// Standard base64 (RFC 4648) alphabet, padded. Hand-rolled rather than a
/// new dependency — the whole point of OSC 52 here is avoiding a
/// system-clipboard crate, and base64 is small and stable enough not to
/// need one either; see the known-vector test below for confidence.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(ALPHABET[(b0 >> 2) as usize] as char);
        out.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(b2 & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// The OSC 52 sequence to copy `text`, wrapped for tmux's DCS passthrough
/// when `in_tmux` — tmux otherwise swallows an arbitrary escape sequence
/// from the program it's running rather than forwarding it to the real
/// terminal underneath. The wrapping is `ESC P tmux ; ESC <sequence> ESC
/// \` with every `ESC` *inside* the original sequence doubled (tmux's DCS
/// parser strips one layer) — here that's just the sequence's own leading
/// `ESC`, so a second one right after the `tmux;` is enough. Split out
/// from `copy_to_system_clipboard` as a pure function so the exact byte
/// layout can be asserted in a test without a real tmux session.
fn osc52_sequence(text: &str, in_tmux: bool) -> String {
    let encoded = base64_encode(text.as_bytes());
    let osc52 = format!("\x1b]52;c;{encoded}\x07");
    if in_tmux {
        format!("\x1bPtmux;\x1b{osc52}\x1b\\")
    } else {
        osc52
    }
}

/// Copies `text` to the system clipboard via an OSC 52 escape sequence
/// written straight to stdout — no OS-level clipboard crate (`arboard` and
/// similar need direct X11/Wayland access), and it works over SSH too,
/// since it's the *client*-side terminal that intercepts the sequence, not
/// the remote shell. Confirmed working under kitty (`Y` in Normal mode).
/// Detects tmux via the `TMUX` env var (set by tmux for every process
/// running inside a session, the same check every other OSC 52 tool uses)
/// and wraps the sequence in tmux's DCS passthrough when present — see
/// `osc52_sequence`.
pub fn copy_to_system_clipboard(text: &str) -> io::Result<()> {
    let in_tmux = std::env::var_os("TMUX").is_some();
    write!(io::stdout(), "{}", osc52_sequence(text, in_tmux))?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encode_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode(b"Hello World"), "SGVsbG8gV29ybGQ=");
    }

    #[test]
    fn osc52_sequence_outside_tmux_is_the_bare_escape_code() {
        assert_eq!(osc52_sequence("hi", false), "\x1b]52;c;aGk=\x07");
    }

    #[test]
    fn osc52_sequence_inside_tmux_gets_dcs_passthrough_wrapping() {
        assert_eq!(
            osc52_sequence("hi", true),
            "\x1bPtmux;\x1b\x1b]52;c;aGk=\x07\x1b\\"
        );
    }
}
