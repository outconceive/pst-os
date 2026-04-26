pub const PLAIN: char = ' ';
pub const BOLD: char = '!';
pub const ITALIC: char = '@';
pub const BOLD_ITALIC: char = '#';
pub const CODE: char = '`';
pub const STRIKETHROUGH: char = '~';
pub const UNDERLINE: char = '_';
pub const LINK: char = '^';
pub const BOLD_CODE: char = '&';
pub const ITALIC_CODE: char = '%';

pub fn is_bold(style: char) -> bool {
    matches!(style, BOLD | BOLD_ITALIC | BOLD_CODE)
}

pub fn is_italic(style: char) -> bool {
    matches!(style, ITALIC | BOLD_ITALIC | ITALIC_CODE)
}

pub fn is_code(style: char) -> bool {
    matches!(style, CODE | BOLD_CODE | ITALIC_CODE)
}

pub fn is_strikethrough(style: char) -> bool {
    style == STRIKETHROUGH
}

pub fn is_underline(style: char) -> bool {
    style == UNDERLINE
}

pub fn is_link(style: char) -> bool {
    style == LINK
}

pub fn add_bold(style: char) -> char {
    match style {
        PLAIN => BOLD,
        ITALIC => BOLD_ITALIC,
        CODE => BOLD_CODE,
        _ => style,
    }
}

pub fn remove_bold(style: char) -> char {
    match style {
        BOLD => PLAIN,
        BOLD_ITALIC => ITALIC,
        BOLD_CODE => CODE,
        _ => style,
    }
}

pub fn toggle_bold(style: char) -> char {
    if is_bold(style) {
        remove_bold(style)
    } else {
        add_bold(style)
    }
}

pub fn add_italic(style: char) -> char {
    match style {
        PLAIN => ITALIC,
        BOLD => BOLD_ITALIC,
        CODE => ITALIC_CODE,
        _ => style,
    }
}

pub fn remove_italic(style: char) -> char {
    match style {
        ITALIC => PLAIN,
        BOLD_ITALIC => BOLD,
        ITALIC_CODE => CODE,
        _ => style,
    }
}

pub fn toggle_italic(style: char) -> char {
    if is_italic(style) {
        remove_italic(style)
    } else {
        add_italic(style)
    }
}

pub fn add_code(style: char) -> char {
    match style {
        PLAIN => CODE,
        BOLD => BOLD_CODE,
        ITALIC => ITALIC_CODE,
        _ => style,
    }
}

pub fn remove_code(style: char) -> char {
    match style {
        CODE => PLAIN,
        BOLD_CODE => BOLD,
        ITALIC_CODE => ITALIC,
        _ => style,
    }
}

pub fn toggle_code(style: char) -> char {
    if is_code(style) {
        remove_code(style)
    } else {
        add_code(style)
    }
}

pub fn combine_styles(a: char, b: char) -> char {
    let mut result = a;
    if is_bold(b) {
        result = add_bold(result);
    }
    if is_italic(b) {
        result = add_italic(result);
    }
    if is_code(b) {
        result = add_code(result);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold_detection() {
        assert!(is_bold(BOLD));
        assert!(is_bold(BOLD_ITALIC));
        assert!(is_bold(BOLD_CODE));
        assert!(!is_bold(PLAIN));
        assert!(!is_bold(ITALIC));
        assert!(!is_bold(CODE));
    }

    #[test]
    fn test_italic_detection() {
        assert!(is_italic(ITALIC));
        assert!(is_italic(BOLD_ITALIC));
        assert!(is_italic(ITALIC_CODE));
        assert!(!is_italic(PLAIN));
        assert!(!is_italic(BOLD));
    }

    #[test]
    fn test_toggle_bold() {
        assert_eq!(toggle_bold(PLAIN), BOLD);
        assert_eq!(toggle_bold(BOLD), PLAIN);
        assert_eq!(toggle_bold(ITALIC), BOLD_ITALIC);
        assert_eq!(toggle_bold(BOLD_ITALIC), ITALIC);
    }

    #[test]
    fn test_toggle_italic() {
        assert_eq!(toggle_italic(PLAIN), ITALIC);
        assert_eq!(toggle_italic(ITALIC), PLAIN);
        assert_eq!(toggle_italic(BOLD), BOLD_ITALIC);
        assert_eq!(toggle_italic(BOLD_ITALIC), BOLD);
    }

    #[test]
    fn test_toggle_code() {
        assert_eq!(toggle_code(PLAIN), CODE);
        assert_eq!(toggle_code(CODE), PLAIN);
        assert_eq!(toggle_code(BOLD), BOLD_CODE);
        assert_eq!(toggle_code(BOLD_CODE), BOLD);
    }

    #[test]
    fn test_combine_styles() {
        assert_eq!(combine_styles(BOLD, ITALIC), BOLD_ITALIC);
        assert_eq!(combine_styles(PLAIN, BOLD), BOLD);
        assert_eq!(combine_styles(BOLD, CODE), BOLD_CODE);
    }
}
