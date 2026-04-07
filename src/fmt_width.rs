use std::fmt::{self, Display, Formatter, Write as _};

use unicode_width::UnicodeWidthStr;

pub struct WidthCounter(usize);

impl WidthCounter {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn get(&self) -> usize {
        self.0
    }
}

impl fmt::Write for WidthCounter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 += s.width();
        Ok(())
    }
}

pub struct FmtFnWrapper<'a, T>(&'a T);

impl<'a, T: Fn(&mut Formatter) -> fmt::Result> FmtFnWrapper<'a, T> {
    pub fn new(fmt_fn: &'a T) -> Self {
        Self(fmt_fn)
    }
}

impl<'a, T: Fn(&mut Formatter) -> fmt::Result> Display for FmtFnWrapper<'a, T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0(f)
    }
}

pub fn width_of<'a, T>(d: &'a T) -> Result<usize, fmt::Error>
where
    &'a T: Display,
{
    let mut width_counter = WidthCounter::new();
    write!(width_counter, "{}", d)?;
    Ok(width_counter.get())
}
