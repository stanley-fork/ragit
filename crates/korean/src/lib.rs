/*
rules

- (?<S>a 한글 term that ends with 종성)(은|이|을|과|으로|이랑|이라고)
- (?<S>a 한글 term that doesn't end with 종성)(는|가|를|와|랑|라고)
- (?<S>[가-힣]+)(의|만|도|에|에서|로|까지|부터|한테|하고|께)
- (?<S>[가-힣]+)(이|하)(ㅂ니다|ㄴ데|ㄴ지|고|면|다|지만)

1. If anything matches, it keeps S and removes suffix.
2. It always tries to match the longest suffix possible.
3. If it fails due to a non-한글 character, it separates non-한글 and 한글 characters and terminate.
4. If it fails due to a 한글 character, it doesn't do anything.
*/

use crate::term_kind::{TermKind, get_term_kind};

mod fsm;
pub mod gen;
mod jamo;
mod hangul;
mod term_kind;

#[cfg(test)]
mod tests;

pub fn tokenize(term: &str) -> Vec<String> {
    match get_term_kind(term) {
        TermKind::No한글(s) => vec![s],
        TermKind::Mixed한글(ts) => ts,
        TermKind::Only한글(js) => vec![fsm::fsm(js)],
    }
}
