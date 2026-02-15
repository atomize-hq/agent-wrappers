#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedChannelString(String);

impl ValidatedChannelString {
    pub const MAX_BYTES: usize = 64;

    pub fn new(raw: &str) -> Option<Self> {
        if raw.is_empty() {
            return None;
        }
        if raw.as_bytes().len() > Self::MAX_BYTES {
            return None;
        }
        if !raw.is_ascii() {
            return None;
        }
        let mut chars = raw.chars();
        let Some(first) = chars.next() else {
            return None;
        };
        if !is_alnum(first) {
            return None;
        }
        for ch in chars {
            if !(is_alnum(ch) || matches!(ch, '.' | '_' | '/' | '-')) {
                return None;
            }
        }
        Some(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_alnum(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
}
