use deunicode::deunicode_char;
// forked from https://github.com/Stebalien/slug-rs
// Added support for two dashes.

pub fn slugify<S: AsRef<str>>(s: S) -> String {
    _slugify(s.as_ref())
}

#[doc(hidden)]
pub fn slugify_owned(s: String) -> String {
    _slugify(s.as_ref())
}

// avoid unnecessary monomorphizations
fn _slugify(s: &str) -> String {
    let mut slug = String::with_capacity(s.len());
    // Starts with true to avoid leading -
    let mut num_dashes = 1;
    {
        let mut push_char = |x: u8| {
            match x {
                b'a'..=b'z' | b'0'..=b'9' => {
                    num_dashes = 0;
                    slug.push(x.into());
                }
                b'A'..=b'Z' => {
                    num_dashes = 0;
                    // Manual lowercasing as Rust to_lowercase() is unicode
                    // aware and therefore much slower
                    slug.push((x - b'A' + b'a').into());
                }
                _ => {
                    if num_dashes < 2 {
                        slug.push('-');
                        num_dashes += 1;
                    }
                }
            }
        };

        for c in s.chars() {
            if c.is_ascii() {
                (push_char)(c as u8);
            } else {
                for &cx in deunicode_char(c).unwrap_or("-").as_bytes() {
                    (push_char)(cx);
                }
            }
        }
    }

    if slug.ends_with('-') {
        slug.pop();
    }
    // We likely reserved more space than needed.
    slug.shrink_to_fit();
    slug
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("hello world"), "hello-world");
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("HeLLo WoRLD"), "hello-world");
    }

    #[test]
    fn test_slugify_two_dashes() {
        assert_eq!(slugify("hello--world"), "hello--world");
        assert_eq!(slugify("hello---world"), "hello--world");
        assert_eq!(slugify("hello & world"), "hello--world");
    }
    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("hello!world"), "hello-world");
        assert_eq!(slugify("hello!!world"), "hello--world");
        assert_eq!(slugify("hello & world"), "hello--world");
        assert_eq!(slugify("hello&world"), "hello-world");
    }

    #[test]
    fn test_slugify_numbers() {
        assert_eq!(slugify("hello123"), "hello123");
        assert_eq!(slugify("123hello"), "123hello");
        assert_eq!(slugify("hello 123 world"), "hello-123-world");
    }

    #[test]
    fn test_slugify_unicode() {
        assert_eq!(slugify("héllo wörld"), "hello-world");
        assert_eq!(slugify("こんにちは"), "konnitiha");
        assert_eq!(slugify("안녕하세요"), "annyeonghaseyo");
    }

    #[test]
    fn test_slugify_edge_cases() {
        assert_eq!(slugify(""), "");
        assert_eq!(slugify("---"), "");
        assert_eq!(slugify("!@#$%"), "");
        assert_eq!(slugify("   "), "");
    }
}
