use std::borrow::Cow;

use bytes::Bytes;
use http_body_util::Full;

use crate::http::HeaderValue;

use super::TypedBody;

impl TypedBody for String {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self.into())
    }
}

impl TypedBody for &'static str {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self.into())
    }
}

impl TypedBody for Cow<'static, str> {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref())
    }

    fn body(self) -> Self::Body {
        match self {
            Cow::Borrowed(s) => s.body(),
            Cow::Owned(s) => s.body(),
        }
    }
}
