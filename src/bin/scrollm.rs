use std::io::Write;

#[derive(Default)]
struct StdoutScroll {
    row: usize,
    reset_once: bool,
}

impl StdoutScroll {
    fn push(&mut self, line: String, write: &mut dyn Write) -> Result<(), std::io::Error> {
        struct Wrapper<'a> {
            rows: usize,
            write: &'a mut dyn Write,
            scroll: &'a mut StdoutScroll,
        }
        impl Write for Wrapper<'_> {
            fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
                assert!(buf.iter().all(|x| *x != b'\n') || buf.len() == 1);
                let num_nl = buf.iter().filter(|x| **x == b'\n').count();
                self.write.write_all(buf)?;
                if num_nl + self.scroll.row >= self.rows {
                    self.scroll.row = 0;
                    self.write.write_all(b"\x1b[H")?;
                    self.scroll.reset_once = true;
                }
                if num_nl > 0 && self.scroll.reset_once {
                    self.write.write_all(b"\x1b[1B\x1b[K\x1b[1A")?;
                }
                self.scroll.row += num_nl;
                Ok(buf.len())
            }

            fn flush(&mut self) -> Result<(), std::io::Error> {
                self.write.flush()
            }
        }
        let (rows, cols) = Self::rows_by_columns();
        let mut wrapper = Wrapper {
            rows,
            write,
            scroll: self,
        };
        let mut w = yammer::WordWrap::new(cols);
        let mut indices_of_whitespace = line
            .char_indices()
            .filter(|(_, c)| c.is_whitespace())
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        indices_of_whitespace.push(line.chars().count());
        w.push(
            line.chars().take(indices_of_whitespace[0]).collect(),
            &mut wrapper,
        )?;
        for slice in std::iter::zip(
            indices_of_whitespace.iter().copied(),
            indices_of_whitespace[1..].iter().copied(),
        ) {
            w.push(
                line.chars().take(slice.1).skip(slice.0).collect(),
                &mut wrapper,
            )?;
            std::thread::sleep(std::time::Duration::from_millis(11));
        }
        w.push("\n".to_string(), &mut wrapper)?;
        Ok(())
    }

    fn rows_by_columns() -> (usize, usize) {
        let mut size = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe { if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) < 0 {} }
        (size.ws_row as usize, size.ws_col as usize)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut wrap = StdoutScroll::default();
    println!("\x1b[2J");
    for line in std::io::stdin().lines() {
        let line = line?;
        wrap.push(line, &mut std::io::stdout())?;
    }
    Ok(())
}
