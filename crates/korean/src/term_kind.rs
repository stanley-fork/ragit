use crate::jamo::{자모, into_자모s};

pub enum TermKind {
    No한글(String),
    Mixed한글(Vec<String>),
    Only한글(Vec<자모>),
}

enum TermKindParseState {
    Init,
    Reading한글,
    ReadingNon한글,
}

pub fn get_term_kind(term: &str) -> TermKind {
    let mut 한글s = vec![];
    let mut no_한글s = vec![];
    let mut curr_state = TermKindParseState::Init;

    for (index, ch) in term.chars().enumerate() {
        match curr_state {
            TermKindParseState::Init => match ch {
                '가'..='힣' => {
                    한글s.push(ch);
                    curr_state = TermKindParseState::Reading한글;
                },
                _ => {
                    no_한글s.push(ch);
                    curr_state = TermKindParseState::ReadingNon한글;
                },
            },
            TermKindParseState::Reading한글 => match ch {
                '가'..='힣' => {
                    한글s.push(ch);
                },
                _ => {
                    let char_vec = term.chars().collect::<Vec<_>>();
                    let (first, second) = char_vec.split_at(index);
                    return TermKind::Mixed한글(vec![
                        first.iter().collect(),
                        second.iter().collect(),
                    ]);
                },
            },
            TermKindParseState::ReadingNon한글 => match ch {
                '가'..='힣' => {
                    let char_vec = term.chars().collect::<Vec<_>>();
                    let (first, second) = char_vec.split_at(index);
                    return TermKind::Mixed한글(vec![
                        first.iter().collect(),
                        second.iter().collect(),
                    ]);
                },
                _ => {
                    no_한글s.push(ch);
                },
            },
        }
    }

    match curr_state {
        TermKindParseState::Init
        | TermKindParseState::ReadingNon한글 => TermKind::No한글(no_한글s.into_iter().collect()),
        TermKindParseState::Reading한글 => TermKind::Only한글(into_자모s(한글s)),
    }
}
