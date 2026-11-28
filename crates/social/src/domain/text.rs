//! What a thread's title and a post's words may be: plain text, of a length, with nothing in it that
//! could hide what it says.

/// Longest and shortest title, in characters.
pub const TITLE_MIN: usize = 3;
pub const TITLE_MAX: usize = 120;
/// Longest post, in characters.
pub const BODY_MAX: usize = 5000;

/// Why some words cannot be posted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextProblem {
    TitleTooShort,
    TitleTooLong,
    BodyEmpty,
    BodyTooLong,
    BadCharacters,
}

impl TextProblem {
    /// A short code for the web app to tell the problems apart.
    pub fn code(self) -> &'static str {
        match self {
            Self::TitleTooShort => "title_too_short",
            Self::TitleTooLong => "title_too_long",
            Self::BodyEmpty => "body_empty",
            Self::BodyTooLong => "body_too_long",
            Self::BadCharacters => "bad_characters",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::TitleTooShort => "A title needs at least 3 characters.",
            Self::TitleTooLong => "A title can have at most 120 characters.",
            Self::BodyEmpty => "Write something first.",
            Self::BodyTooLong => "A post can have at most 5000 characters.",
            Self::BadCharacters => "That text has characters that cannot be posted.",
        }
    }
}

/// Characters that change how text reads rather than what it says: the direction overrides that
/// can make one thing look like another, and zero-width characters that make two names equal to the
/// eye and different to the machine.
fn is_hidden_control(c: char) -> bool {
    matches!(
        c,
        '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}' | '\u{FEFF}'
    )
}

/// A thread's title: one line, trimmed.
pub fn title(raw: &str) -> Result<String, TextProblem> {
    let title = raw.trim();
    if title
        .chars()
        .any(|c| c.is_control() || is_hidden_control(c))
    {
        return Err(TextProblem::BadCharacters);
    }
    let length = title.chars().count();
    if length < TITLE_MIN {
        return Err(TextProblem::TitleTooShort);
    }
    if length > TITLE_MAX {
        return Err(TextProblem::TitleTooLong);
    }
    Ok(title.to_string())
}

/// A post's words: line breaks kept (as `\n`), trimmed. Tabs are allowed, other control
/// characters are not.
pub fn body(raw: &str) -> Result<String, TextProblem> {
    let text = raw.replace("\r\n", "\n").replace('\r', "\n");
    let text = text.trim();
    if text
        .chars()
        .any(|c| (c.is_control() && c != '\n' && c != '\t') || is_hidden_control(c))
    {
        return Err(TextProblem::BadCharacters);
    }
    let length = text.chars().count();
    if length == 0 {
        return Err(TextProblem::BodyEmpty);
    }
    if length > BODY_MAX {
        return Err(TextProblem::BodyTooLong);
    }
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_is_trimmed_and_measured_in_characters() {
        assert_eq!(title("  Opening ideas  ").unwrap(), "Opening ideas");
        assert_eq!(title("ab"), Err(TextProblem::TitleTooShort));
        assert_eq!(title("   a   "), Err(TextProblem::TitleTooShort));
        assert!(title(&"é".repeat(120)).is_ok());
        assert_eq!(title(&"é".repeat(121)), Err(TextProblem::TitleTooLong));
    }

    #[test]
    fn a_title_is_one_line() {
        assert_eq!(title("two\nlines"), Err(TextProblem::BadCharacters));
        assert_eq!(title("tab\there"), Err(TextProblem::BadCharacters));
    }

    #[test]
    fn a_body_keeps_its_line_breaks() {
        assert_eq!(body("one\r\ntwo\rthree\n").unwrap(), "one\ntwo\nthree");
        assert_eq!(body("a\tb").unwrap(), "a\tb");
    }

    #[test]
    fn a_body_cannot_be_empty_or_too_long() {
        assert_eq!(body("  \n "), Err(TextProblem::BodyEmpty));
        assert!(body(&"x".repeat(5000)).is_ok());
        assert_eq!(body(&"x".repeat(5001)), Err(TextProblem::BodyTooLong));
    }

    #[test]
    fn controls_and_direction_overrides_are_refused() {
        assert_eq!(body("a\u{0007}b"), Err(TextProblem::BadCharacters));
        assert_eq!(body("a\u{202E}b"), Err(TextProblem::BadCharacters));
        assert_eq!(title("na\u{200B}me"), Err(TextProblem::BadCharacters));
    }
}
