//! Minimal JSON parser/serializer (std-only).
//!
//! Used for three things: decoding incoming commands and save data, encoding
//! outgoing DOM patches, and writing save files in a format shaped like the
//! original The Modding Tree saves (objects keep insertion order).

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    // ---------- constructors ----------

    pub fn s(v: impl Into<String>) -> Json {
        Json::Str(v.into())
    }

    pub fn obj(fields: Vec<(&str, Json)>) -> Json {
        Json::Obj(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn arr(items: Vec<Json>) -> Json {
        Json::Arr(items)
    }

    // ---------- accessors ----------

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(o) => Some(o),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }

    /// String content if this is a string, otherwise `None`.
    pub fn str_or(&self, default: &str) -> String {
        self.as_str().unwrap_or(default).to_string()
    }

    /// Number as f64; accepts a numeric string too (save files may hold either).
    pub fn num_or(&self, default: f64) -> f64 {
        match self {
            Json::Num(n) => *n,
            Json::Str(s) => s.trim().parse().unwrap_or(default),
            _ => default,
        }
    }

    // ---------- serialization ----------

    pub fn to_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(true) => out.push_str("true"),
            Json::Bool(false) => out.push_str("false"),
            Json::Num(n) => out.push_str(&num_to_json_string(*n)),
            Json::Str(s) => write_escaped(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_escaped(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    // ---------- parsing ----------

    pub fn parse(input: &str) -> Result<Json, String> {
        let bytes = input.as_bytes();
        let mut pos = 0usize;
        let value = parse_value(bytes, &mut pos)?;
        skip_ws(bytes, &mut pos);
        if pos != bytes.len() {
            return Err(format!("trailing characters at byte {}", pos));
        }
        Ok(value)
    }
}

/// Format a number the way JSON.stringify would (no trailing ".0").
pub fn num_to_json_string(n: f64) -> String {
    if !n.is_finite() {
        // JSON.stringify turns these into null; saves should never contain them,
        // but be defensive rather than emit invalid JSON.
        return "null".to_string();
    }
    if n == 0.0 {
        return "0".to_string();
    }
    if n.fract() == 0.0 && n.abs() < 1e21 {
        return format!("{}", n as i64);
    }
    format!("{}", n)
}

fn write_escaped(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn skip_ws(bytes: &[u8], pos: &mut usize) {
    while *pos < bytes.len() {
        match bytes[*pos] {
            b' ' | b'\t' | b'\n' | b'\r' => *pos += 1,
            _ => break,
        }
    }
}

fn parse_value(bytes: &[u8], pos: &mut usize) -> Result<Json, String> {
    skip_ws(bytes, pos);
    if *pos >= bytes.len() {
        return Err("unexpected end of input".to_string());
    }
    match bytes[*pos] {
        b'{' => parse_object(bytes, pos),
        b'[' => parse_array(bytes, pos),
        b'"' => Ok(Json::Str(parse_string(bytes, pos)?)),
        b't' => parse_lit(bytes, pos, "true", Json::Bool(true)),
        b'f' => parse_lit(bytes, pos, "false", Json::Bool(false)),
        b'n' => parse_lit(bytes, pos, "null", Json::Null),
        b'-' | b'0'..=b'9' => parse_number(bytes, pos),
        c => Err(format!("unexpected character '{}' at byte {}", c as char, pos)),
    }
}

fn parse_number(bytes: &[u8], pos: &mut usize) -> Result<Json, String> {
    let start = *pos;
    if bytes[*pos] == b'-' {
        *pos += 1;
    }
    while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
        *pos += 1;
    }
    // fractional part
    if *pos < bytes.len() && bytes[*pos] == b'.' {
        *pos += 1;
        while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
            *pos += 1;
        }
    }
    // exponent
    if *pos < bytes.len() && (bytes[*pos] == b'e' || bytes[*pos] == b'E') {
        let mut next = *pos + 1;
        if next < bytes.len() && (bytes[next] == b'-' || bytes[next] == b'+') {
            next += 1;
        }
        if next < bytes.len() && bytes[next].is_ascii_digit() {
            *pos = next;
            while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
                *pos += 1;
            }
        }
    }
    let text = std::str::from_utf8(&bytes[start..*pos]).map_err(|_| "bad number".to_string())?;
    text.parse::<f64>()
        .map(Json::Num)
        .map_err(|_| format!("invalid number '{}' at byte {}", text, start))
}

fn parse_lit(bytes: &[u8], pos: &mut usize, lit: &str, value: Json) -> Result<Json, String> {
    if bytes[*pos..].starts_with(lit.as_bytes()) {
        *pos += lit.len();
        Ok(value)
    } else {
        Err(format!("invalid literal at byte {}", pos))
    }
}

fn parse_object(bytes: &[u8], pos: &mut usize) -> Result<Json, String> {
    *pos += 1; // '{'
    let mut fields = Vec::new();
    skip_ws(bytes, pos);
    if *pos < bytes.len() && bytes[*pos] == b'}' {
        *pos += 1;
        return Ok(Json::Obj(fields));
    }
    loop {
        skip_ws(bytes, pos);
        if *pos >= bytes.len() || bytes[*pos] != b'"' {
            return Err(format!("expected object key at byte {}", pos));
        }
        let key = parse_string(bytes, pos)?;
        skip_ws(bytes, pos);
        if *pos >= bytes.len() || bytes[*pos] != b':' {
            return Err(format!("expected ':' at byte {}", pos));
        }
        *pos += 1;
        let value = parse_value(bytes, pos)?;
        fields.push((key, value));
        skip_ws(bytes, pos);
        if *pos >= bytes.len() {
            return Err("unterminated object".to_string());
        }
        match bytes[*pos] {
            b',' => *pos += 1,
            b'}' => {
                *pos += 1;
                return Ok(Json::Obj(fields));
            }
            c => return Err(format!("expected ',' or '}}', got '{}' at byte {}", c as char, pos)),
        }
    }
}

fn parse_array(bytes: &[u8], pos: &mut usize) -> Result<Json, String> {
    *pos += 1; // '['
    let mut items = Vec::new();
    skip_ws(bytes, pos);
    if *pos < bytes.len() && bytes[*pos] == b']' {
        *pos += 1;
        return Ok(Json::Arr(items));
    }
    loop {
        let value = parse_value(bytes, pos)?;
        items.push(value);
        skip_ws(bytes, pos);
        if *pos >= bytes.len() {
            return Err("unterminated array".to_string());
        }
        match bytes[*pos] {
            b',' => *pos += 1,
            b']' => {
                *pos += 1;
                return Ok(Json::Arr(items));
            }
            c => return Err(format!("expected ',' or ']', got '{}' at byte {}", c as char, pos)),
        }
    }
}

fn parse_string(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    *pos += 1; // opening quote
    let mut out = String::new();
    loop {
        if *pos >= bytes.len() {
            return Err("unterminated string".to_string());
        }
        match bytes[*pos] {
            b'"' => {
                *pos += 1;
                return Ok(out);
            }
            b'\\' => {
                *pos += 1;
                if *pos >= bytes.len() {
                    return Err("unterminated escape".to_string());
                }
                match bytes[*pos] {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    b'b' => out.push('\u{08}'),
                    b'f' => out.push('\u{0C}'),
                    b'u' => {
                        // `pos` is on the 'u'; parse_hex4 leaves it on the
                        // last hex digit.
                        let hi = parse_hex4(bytes, pos)?;
                        let cp = if (0xD800..0xDC00).contains(&hi)
                            && *pos + 2 < bytes.len()
                            && bytes[*pos + 1] == b'\\'
                            && bytes[*pos + 2] == b'u'
                        {
                            // Skip "\u" of the low surrogate (pos + 2 is 'u').
                            *pos += 2;
                            let lo = parse_hex4(bytes, pos)?;
                            if (0xDC00..0xE000).contains(&lo) {
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                0xFFFD
                            }
                        } else if (0xDC00..0xE000).contains(&hi) {
                            0xFFFD // lone low surrogate
                        } else {
                            hi
                        };
                        out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                    }
                    c => return Err(format!("bad escape '\\{}'", c as char)),
                }
                *pos += 1;
            }
            _ => {
                // Copy a whole UTF-8 sequence at once.
                let start = *pos;
                let len = utf8_len(bytes[start]);
                *pos += len;
                if *pos > bytes.len() {
                    return Err("truncated UTF-8".to_string());
                }
                match std::str::from_utf8(&bytes[start..*pos]) {
                    Ok(s) => out.push_str(s),
                    Err(_) => out.push('\u{FFFD}'),
                }
            }
        }
    }
}

fn parse_hex4(bytes: &[u8], pos: &mut usize) -> Result<u32, String> {
    // `pos` points at the 'u'; parse the 4 hex digits after it.
    let mut v: u32 = 0;
    for _ in 0..4 {
        *pos += 1;
        if *pos >= bytes.len() {
            return Err("truncated \\u escape".to_string());
        }
        let d = match bytes[*pos] {
            c @ b'0'..=b'9' => (c - b'0') as u32,
            c @ b'a'..=b'f' => (c - b'a' + 10) as u32,
            c @ b'A'..=b'F' => (c - b'A' + 10) as u32,
            _ => return Err("bad hex digit in \\u escape".to_string()),
        };
        v = v * 16 + d;
    }
    Ok(v)
}

fn utf8_len(byte: u8) -> usize {
    if byte < 0x80 {
        1
    } else if byte >> 5 == 0b110 {
        2
    } else if byte >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let src = r#"{"a":1,"b":[true,null,"x\ny"],"c":{"d":-2.5},"e":"中文"}"#;
        let v = Json::parse(src).unwrap();
        assert_eq!(v.get("a").unwrap().as_f64(), Some(1.0));
        assert_eq!(v.get("e").unwrap().as_str(), Some("中文"));
        let out = v.to_string();
        let v2 = Json::parse(&out).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn numbers() {
        assert_eq!(num_to_json_string(1.0), "1");
        assert_eq!(num_to_json_string(-0.5), "-0.5");
        assert_eq!(num_to_json_string(1e21), "1000000000000000000000");
    }

    #[test]
    fn surrogate_pair() {
        let v = Json::parse("\"\\ud83d\\ude00\"").unwrap();
        assert_eq!(v.as_str(), Some("😀"));
    }
}
