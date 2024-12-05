use std::io::Write;

/// A word-wrapping struct.
#[derive(Debug)]
pub struct WordWrap {
    width: usize,
    indent: Vec<usize>,
    current_line: String,
}

impl WordWrap {
    /// Create a new word-wrapping struct that wraps at `width`.
    pub fn new(mut width: usize) -> Self {
        if width == 0 {
            width = usize::MAX;
        }
        Self {
            width,
            indent: vec![],
            current_line: String::new(),
        }
    }

    /// Push a token onto the output and word-wrap it to tee.
    pub fn push(&mut self, tok: String, tee: &mut impl Write) -> Result<(), std::io::Error> {
        if tok.is_empty() {
            return Ok(());
        }
        if tok != "\n" && tok.contains('\n') {
            let toks = tok.split('\n').collect::<Vec<_>>();
            for (i, tok) in toks.iter().enumerate() {
                if i == 0 {
                    self.push(tok.to_string(), tee)?;
                } else {
                    self.push("\n".to_string(), tee)?;
                    self.push(tok.to_string(), tee)?;
                }
            }
            return Ok(());
        }
        if tok == "\n" {
            tee.write_all(tok.as_bytes())?;
            tee.flush()?;
            self.current_line.clear();
        } else if self.current_line.chars().count() + tok.chars().count() > self.width
            && tok.chars().count() <= self.width
            && tok.chars().count() > tok.trim_start().chars().count()
        {
            self.current_line.clear();
            tee.write_all("\n".as_bytes())?;
            self.push(
                " ".repeat(*self.indent.iter().last().unwrap_or(&0))
                    .to_string()
                    + tok.trim_start(),
                tee,
            )?;
        } else if self.current_line.chars().all(|c| c.is_whitespace()) {
            self.current_line += &tok;
            if !self.current_line.chars().all(|c| c.is_whitespace()) {
                let leading_whitespace = self.current_line.chars().count()
                    - self.current_line.trim_start().chars().count();
                while !self.indent.is_empty() && *self.indent.last().unwrap() >= leading_whitespace
                {
                    self.indent.pop();
                }
                self.indent.push(leading_whitespace);
                self.current_line = " ".repeat(self.indent.len() * 2 - 2).to_string();
                self.current_line += tok.trim_start();
                tee.write_all(self.current_line.as_bytes())?;
                tee.flush()?;
            }
        } else {
            self.current_line += &tok;
            tee.write_all(tok.as_bytes())?;
            tee.flush()?;
        }
        Ok(())
    }
}
