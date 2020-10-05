use crossterm::style::Colorize;
use crossterm::*;

use std::cmp::min;
use std::io::{stderr, Write};
use tokio::time::Duration;

fn pack_into<F: FnMut() -> u8>(buf: &mut [u8], f: &mut F) {
    for ch in buf.iter_mut() {
        *ch = f()
    }
}

pub fn fade_in(banner: &str) -> anyhow::Result<()> {
    let mut stderr = stderr();
    let noise = b"@Oo*.";
    let mut seed = 1000usize;
    let mut noise_buf = [b' '; 5];
    let mut next_noise_char = move || -> u8 {
        let n = seed % noise.len();
        seed = (seed * 13 + 7) & 0xFFFF;
        noise[n]
    };

    queue!(stderr, cursor::Hide)?;
    for frame in 0.. {
        let mut nlines = 0;
        let mut next_frame: bool = false;
        for line in banner.lines() {
            let offset = if 5 + (frame / 3) > nlines {
                5 + (frame / 3) - nlines
            } else {
                0
            };

            let (pre, post) = if line.len() > offset {
                next_frame = true;
                (&line[..offset], &line[offset..])
            } else {
                (line, "")
            };
            let post = if post.is_empty() {
                ("", "")
            } else {
                pack_into(noise_buf.as_mut(), &mut next_noise_char);
                (
                    std::str::from_utf8(&noise_buf[..min(noise_buf.len(), post.len())])?,
                    &post[1..],
                )
            };
            queue!(
                stderr,
                style::Print(pre),
                style::PrintStyledContent(post.0.red()),
                style::PrintStyledContent(post.1.black()),
                style::Print("\n")
            )?;
            nlines += 1;
        }
        stderr.flush()?;
        if next_frame {
            queue!(stderr, cursor::MoveToPreviousLine(nlines as u16))?;
        } else {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    queue!(stderr, cursor::Show)?;
    stderr.flush()?;
    Ok(())
}

pub async fn clear_stdin() -> anyhow::Result<()> {
    let _ = crossterm::terminal::enable_raw_mode()?;
    while crossterm::event::poll(Duration::from_millis(100))? {
        let _ = crossterm::event::read()?;
    }
    let _ = crossterm::terminal::disable_raw_mode()?;
    Ok(())
}
