#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceButton { A, B, X, Y }

pub fn label(symbol: Option<&str>, name: Option<&str>) -> Option<FaceButton> {
    use FaceButton::*;
    // Use the symbol for the physical button that supplies this mapped input.
    // In particular, PlayStation's xmark symbol is not the letter X.
    match symbol {
        Some("a.circle" | "a.circle.fill") => return Some(A),
        Some("b.circle" | "b.circle.fill") => return Some(B),
        Some("x.circle" | "x.circle.fill") => return Some(X),
        Some("y.circle" | "y.circle.fill") => return Some(Y),
        _ => {}
    }
    let mut letters = name?.split(|c: char| !c.is_ascii_alphanumeric()).filter_map(|word| match word {
        "A" | "a" => Some(A), "B" | "b" => Some(B),
        "X" | "x" => Some(X), "Y" | "y" => Some(Y), _ => None,
    });
    let first = letters.next()?;
    if letters.next().is_some() { None } else { Some(first) }
}

#[cfg(test)]
mod tests {
    use super::{FaceButton::*, *};

    #[test]
    fn xbox_and_switch_labels_follow_their_respective_positions() {
        assert_eq!(["a.circle", "b.circle", "x.circle", "y.circle"].map(|s| label(Some(s), None)),
            [Some(A), Some(B), Some(X), Some(Y)]);
        assert_eq!(["b.circle", "a.circle", "y.circle", "x.circle"].map(|s| label(Some(s), None)),
            [Some(B), Some(A), Some(Y), Some(X)]);
    }

    #[test]
    fn remapped_symbol_takes_precedence_over_logical_name() {
        assert_eq!(label(Some("b.circle.fill"), Some("Button A")), Some(B));
        assert_eq!(label(Some("y.circle"), Some("Button X")), Some(Y));
    }

    #[test]
    fn localized_names_work_without_symbols() {
        assert_eq!(label(None, Some("A 按钮")), Some(A));
        assert_eq!(label(None, Some("按钮B")), Some(B));
        assert_eq!(label(None, Some("Button X")), Some(X));
        assert_eq!(label(None, Some("Y-Taste")), Some(Y));
    }

    #[test]
    fn unknown_or_ambiguous_labels_do_not_guess() {
        assert_eq!(label(Some("xmark.circle"), Some("Cross")), None);
        assert_eq!(label(None, Some("Button A / Button B")), None);
        assert_eq!(label(None, Some("Xbox")), None);
        assert_eq!(label(None, None), None);
    }
}
