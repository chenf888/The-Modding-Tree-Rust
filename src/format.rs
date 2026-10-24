//! Number formatting — a direct port of js/utils/NumberFormating.js from The
//! Modding Tree, so every displayed value looks exactly like the original.

pub use crate::{Decimal, TetrationMode};

/// Parse a Decimal from a string. Mirrors JS `new Decimal(str)` semantics:
/// anything unparseable becomes 0 (the original template never throws here).
pub fn dec(s: &str) -> Decimal {
    s.parse().unwrap_or(Decimal::zero())
}

/// Mirror of JS `new Decimal(number)`: ±inf map to the canonical infinities,
/// NaN to the NaN sentinel, everything else to a normalized Decimal.
pub fn dnum(n: f64) -> Decimal {
    if n.is_nan() {
        Decimal::from_components(1, 0, f64::NAN)
    } else if n == f64::INFINITY {
        Decimal::inf()
    } else if n == f64::NEG_INFINITY {
        Decimal::neg_inf()
    } else {
        Decimal::from_finite(n)
    }
}


/// Decimal::to_string_with_decimal_places with JS layer-0 semantics: plain
/// numbers are formatted with the JS toFixed tie rule (ties toward +infinity),
/// which the crate's Display rounding does not reproduce.
fn js_dws(d: &Decimal, places: i32) -> String {
    if d.layer() == 0 && ((d.mag() < 1e21 && d.mag() > 1e-7) || d.mag() == 0.0) {
        return js_to_fixed(d.sign() as f64 * d.mag(), places);
    }
    d.to_string_with_decimal_places(places.max(0) as usize, Some(true))
}

fn pow10(d: Decimal) -> Decimal {
    Decimal::from(10).pow(d)
}

fn exponential_format(num: &Decimal, precision: i32, mantissa: bool) -> String {
    let mut e = num.log10().floor();
    let mut m = *num / pow10(e);
    // JS compares the *string* produced by toStringWithDecimalPlaces against
    // the number 10 with loose equality (string -> number coercion).
    let m_str = js_dws(&m, precision);
    if m_str.parse::<f64>().unwrap_or(0.0) == 10.0 {
        m = Decimal::from(1);
        e = e + Decimal::from(1);
    }
    let e_str = if e >= dnum(1e9) {
        format(e, 3, false)
    } else if e >= Decimal::from(10000) {
        comma_format(&e, 0)
    } else {
        e.to_string_with_decimal_places(0, Some(true))
    };
    if mantissa {
        format!("{}e{}", js_dws(&m, precision), e_str)
    } else {
        format!("e{}", e_str)
    }
}

fn comma_format(num: &Decimal, precision: i32) -> String {
    if num.mag() < 0.001 {
        return js_to_fixed(0.0, precision);
    }
    let init = js_dws(num, precision);
    let mut portions: Vec<String> = init.split('.').map(|s| s.to_string()).collect();
    if portions.is_empty() {
        return init;
    }
    portions[0] = insert_commas(&portions[0]);
    if portions.len() == 1 {
        return portions[0].clone();
    }
    format!("{}.{}", portions[0], portions[1])
}

fn insert_commas(digits: &str) -> String {
    // Equivalent of /(\d)(?=(\d\d\d)+(?!\d))/g -> "$1,"
    let chars: Vec<char> = digits.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(n + n / 3);
    for (i, c) in chars.iter().enumerate() {
        if c.is_ascii_digit() && (n - i - 1) % 3 == 0 && i != n - 1 && chars[i + 1].is_ascii_digit() {
            out.push(*c);
            out.push(',');
        } else {
            out.push(*c);
        }
    }
    out
}

fn regular_format(num: &Decimal, precision: i32) -> String {
    if num.mag() < 0.0001 {
        return js_to_fixed(0.0, precision);
    }
    let precision = if num.mag() < 0.1 && precision != 0 {
        precision.max(4)
    } else {
        precision
    };
    js_dws(num, precision.max(0))
}

/// JS-compatible Number.prototype.toFixed for the values the game shows.
pub fn js_to_fixed(x: f64, places: i32) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if !x.is_finite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if x.abs() >= 1e21 {
        return x.to_string();
    }
    let m = 10f64.powi(places);
    // JS picks the larger n when x*10^p is exactly halfway, i.e. ties round
    // toward +infinity: floor(y + 0.5).
    let v = (x * m + 0.5).floor() / m;
    let s = format!("{:.*}", places.max(0) as usize, v);
    if v == 0.0 && s.starts_with('-') {
        s[1..].to_string()
    } else {
        s
    }
}

pub fn format(decimal: Decimal, precision: i32, small: bool) -> String {
    // The vendored Decimal stores NaN in `mag` (sign/layer are integers).
    if decimal.mag().is_nan() {
        return "NaN".to_string();
    }
    if decimal.sign() < 0 {
        return format!("-{}", format(-decimal, precision, small));
    }
    if decimal.mag() == f64::INFINITY {
        return "Infinity".to_string();
    }
    let eeee1000 = dec("eeee1000");
    if decimal >= eeee1000 {
        let slog = decimal.slog(None, TetrationMode::Analytic);
        if slog >= dnum(1e6) {
            return format!("F{}", format(slog.floor(), 3, false));
        } else {
            return format!(
                "{}F{}",
                pow10(slog - slog.floor()).to_string_with_decimal_places(3, Some(true)),
                comma_format(&slog.floor(), 0)
            );
        }
    } else if decimal >= dec("1e1000000") {
        return exponential_format(&decimal, 0, false);
    } else if decimal >= dec("1e10000") {
        return exponential_format(&decimal, 0, true);
    } else if decimal >= dnum(1e9) {
        return exponential_format(&decimal, precision, true);
    } else if decimal >= dnum(1e3) {
        return comma_format(&decimal, 0);
    } else if decimal >= dnum(0.0001) || !small {
        return regular_format(&decimal, precision);
    } else if decimal == Decimal::zero() {
        return js_to_fixed(0.0, precision);
    }

    let decimal = invert_oom(decimal);
    if decimal < dec("1e1000") {
        let val = exponential_format(&decimal, precision, true);
        // JS: val.replace(/([^(?:e|F)]*)$/, '-$1') — prepend '-' to the
        // trailing run of characters that are neither 'e' nor 'F'.
        let suffix_len = val
            .chars()
            .rev()
            .take_while(|c| *c != 'e' && *c != 'F')
            .count();
        let split = val.len() - suffix_len;
        format!("{}-{}", &val[..split], &val[split..])
    } else {
        format!("{}⁻¹", format(decimal, precision, small))
    }
}

pub fn format_whole(decimal: Decimal) -> String {
    if decimal >= dnum(1e9) {
        return format(decimal, 2, false);
    }
    if decimal <= dnum(0.99) && decimal != Decimal::zero() {
        return format(decimal, 2, false);
    }
    format(decimal, 0, false)
}

pub fn format_time(s: f64) -> String {
    if s < 60.0 {
        return format!("{}s", format(dnum(s), 2, false));
    } else if s < 3600.0 {
        return format!(
            "{}m {}s",
            format_whole(dnum((s / 60.0).floor())),
            format(dnum(s % 60.0), 2, false)
        );
    } else if s < 86400.0 {
        return format!(
            "{}h {}m {}s",
            format_whole(dnum((s / 3600.0).floor())),
            format_whole(dnum((s / 60.0).floor() % 60.0)),
            format(dnum(s % 60.0), 2, false)
        );
    } else if s < 31536000.0 {
        return format!(
            "{}d {}h {}m {}s",
            format_whole(dnum((s / 86400.0).floor() % 365.0)),
            format_whole(dnum((s / 3600.0).floor() % 24.0)),
            format_whole(dnum((s / 60.0).floor() % 60.0)),
            format(dnum(s % 60.0), 2, false)
        );
    } else {
        format!(
            "{}y {}d {}h {}m {}s",
            format_whole(dnum((s / 31536000.0).floor())),
            format_whole(dnum((s / 86400.0).floor() % 365.0)),
            format_whole(dnum((s / 3600.0).floor() % 24.0)),
            format_whole(dnum((s / 60.0).floor() % 60.0)),
            format(dnum(s % 60.0), 2, false)
        )
    }
}

pub fn format_small(x: Decimal, precision: i32) -> String {
    format(x, precision, true)
}

pub fn to_places(x: Decimal, precision: usize, max_accepted: Decimal) -> String {
    let mut result = x.to_string_with_decimal_places(precision, Some(true));
    if dec(&result) >= max_accepted {
        let capped = max_accepted.to_number() - 10f64.powi(-(precision as i32));
        result = dnum(capped).to_string_with_decimal_places(precision, Some(true));
    }
    result
}

fn invert_oom(x: Decimal) -> Decimal {
    let mut e = x.log10().ceil();
    let m = x / pow10(e);
    e = -e;
    pow10(e) * m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_format() {
        // Expected values verified against the original JS formatting rules:
        // regularFormat pads with the given precision, commaFormat truncates
        // to whole numbers, exponentialFormat keeps the precision digits.
        assert_eq!(format(dec("0"), 2, false), "0.00");
        assert_eq!(format(dec("10"), 2, false), "10.00");
        assert_eq!(format(dec("1234.5"), 2, false), "1,235");
        assert_eq!(format(dec("12345678"), 2, false), "12,345,678");
        assert_eq!(format(dec("1234567890"), 2, false), "1.23e9");
        assert_eq!(format(dec("1e1000"), 2, false), "1.00e1000");
        assert_eq!(format(dec("-5"), 2, false), "-5.00");
        assert_eq!(format_whole(dec("1e10")), "1.00e10");
        assert_eq!(format_whole(dec("7")), "7");
    }

    #[test]
    fn small_format() {
        // regularFormat raises the precision to at least 4 below 0.1.
        assert_eq!(format(dec("0.001"), 2, true), "0.0010");
        assert_eq!(format(dec("0.001234"), 2, true), "0.0012");
        // Below 1e-4 the number is inverted and shown with a negative exponent.
        assert_eq!(format(dec("0.00001"), 2, true), "1.00e-5");
    }

    #[test]
    fn time_format() {
        assert_eq!(format_time(30.0), "30.00s");
        assert_eq!(format_time(90.0), "1m 30.00s");
        assert_eq!(format_time(3661.0), "1h 1m 1.00s");
    }
}
