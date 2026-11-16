//! Pure helpers for the "Select all" control on the New Bill screen.
//!
//! The mobile layer keeps a list of picks and renders a tri-state checkbox
//! from these helpers so the behaviour is covered by core tests.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionState {
    None,
    Some,
    All,
}

/// Empty or all-false selections are `None`; every row selected is `All`;
/// anything in between is `Some`.
pub fn selection_state(selected: &[bool]) -> SelectionState {
    if selected.is_empty() || selected.iter().all(|flag| !*flag) {
        return SelectionState::None;
    }
    if selected.iter().all(|flag| *flag) {
        return SelectionState::All;
    }
    SelectionState::Some
}

/// Select-all transition: everything selected becomes nothing selected;
/// none or some becomes everything selected. Length and order are preserved.
pub fn toggle_select_all(selected: &[bool]) -> Vec<bool> {
    let select = selection_state(selected) != SelectionState::All;
    selected.iter().map(|_| select).collect()
}

pub fn selected_count(selected: &[bool]) -> usize {
    selected.iter().filter(|flag| **flag).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_all_false_are_none() {
        assert_eq!(selection_state(&[]), SelectionState::None);
        assert_eq!(selection_state(&[false, false]), SelectionState::None);
    }

    #[test]
    fn mixed_selection_is_some() {
        assert_eq!(selection_state(&[true, false]), SelectionState::Some);
        assert_eq!(
            selection_state(&[false, true, true]),
            SelectionState::Some
        );
    }

    #[test]
    fn all_true_is_all() {
        assert_eq!(selection_state(&[true]), SelectionState::All);
        assert_eq!(selection_state(&[true, true, true]), SelectionState::All);
    }

    #[test]
    fn toggle_selects_everything_from_none_or_some() {
        assert_eq!(toggle_select_all(&[]), Vec::<bool>::new());
        assert_eq!(toggle_select_all(&[false, false]), vec![true, true]);
        assert_eq!(toggle_select_all(&[true, false, false]), vec![true, true, true]);
    }

    #[test]
    fn toggle_clears_everything_from_all() {
        assert_eq!(toggle_select_all(&[true, true, true]), vec![false, false, false]);
    }

    #[test]
    fn selected_count_matches() {
        assert_eq!(selected_count(&[]), 0);
        assert_eq!(selected_count(&[false, true, true]), 2);
        assert_eq!(selected_count(&[true, true]), 2);
    }
}
