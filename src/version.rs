use std::borrow::Cow;

#[derive(Debug)]
pub struct RawVersion<'a>(Cow<'a, str>);

impl<'a> From<Cow<'a, str>> for RawVersion<'a> {
    fn from(s: Cow<'a, str>) -> Self {
        Self(s)
    }
}

#[derive(Debug)]
pub struct Version<'a>(Cow<'a, str>);

impl<'a> Version<'a> {
    pub fn from_raw_str(raw: Cow<'a, str>, strip_v_prefix: bool) -> Self {
        if strip_v_prefix && raw.starts_with('v') {
            Self(Cow::Owned(raw[1..].to_owned()))
        } else {
            Self(raw)
        }
    }

    pub fn from_raw(raw: RawVersion<'a>, strip_v_prefix: bool) -> Self {
        Self::from_raw_str(raw.0, strip_v_prefix)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}
