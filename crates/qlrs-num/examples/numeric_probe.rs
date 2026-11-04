//! Line-oriented arithmetic probe for the independent Python integer oracle.
#![forbid(unsafe_code)]

use std::io::{self, BufRead, BufWriter, Write};

use qlrs_num::{Fixed, Rounding};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let line = line?;
        let mut fields = line.split_whitespace();
        let left_raw: i128 = fields.next().ok_or("missing left raw")?.parse()?;
        let left_scale: u8 = fields.next().ok_or("missing left scale")?.parse()?;
        let right_raw: i128 = fields.next().ok_or("missing right raw")?.parse()?;
        let right_scale: u8 = fields.next().ok_or("missing right scale")?.parse()?;
        let target: u8 = fields.next().ok_or("missing target scale")?.parse()?;
        let rounding = match fields.next().ok_or("missing rounding")? {
            "Exact" => Rounding::Exact,
            "HalfEven" => Rounding::HalfEven,
            "HalfUp" => Rounding::HalfUp,
            "Down" => Rounding::Down,
            _ => return Err("unknown rounding".into()),
        };
        let left = Fixed::new(left_raw, left_scale)?;
        let right = Fixed::new(right_raw, right_scale)?;
        match left.mul_rescale(right, target, rounding) {
            Ok(value) => writeln!(output, "{} {}", value.raw(), value.scale())?,
            Err(error) => writeln!(output, "{error}")?,
        }
    }
    Ok(())
}
