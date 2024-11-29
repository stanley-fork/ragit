use crate::jamo::{자모, into_자모s};

pub enum TokenType {
    No한글(String),
    Mixed한글(Vec<String>),
    Only한글(Vec<자모>),
}

enum TokenTypeParseState {
    Init,
    Reading한글,
    ReadingNon한글,
}

pub fn get_token_type(token: &str) -> TokenType {
    let mut 한글s = vec![];
    let mut no_한글s = vec![];
    let mut curr_state = TokenTypeParseState::Init;

    for (index, ch) in token.chars().enumerate() {
        match curr_state {
            TokenTypeParseState::Init => match ch {
                '가'..='힣' => {
                    한글s.push(ch);
                    curr_state = TokenTypeParseState::Reading한글;
                },
                _ => {
                    no_한글s.push(ch);
                    curr_state = TokenTypeParseState::ReadingNon한글;
                },
            },
            TokenTypeParseState::Reading한글 => match ch {
                '가'..='힣' => {
                    한글s.push(ch);
                },
                _ => {
                    return TokenType::Mixed한글(vec![
                        token.get(..index).unwrap().to_string(),
                        token.get(index..).unwrap().to_string(),
                    ]);
                },
            },
            TokenTypeParseState::ReadingNon한글 => match ch {
                '가'..='힣' => {
                    return TokenType::Mixed한글(vec![
                        token.get(..index).unwrap().to_string(),
                        token.get(index..).unwrap().to_string(),
                    ]);
                },
                _ => {
                    no_한글s.push(ch);
                },
            },
        }
    }

    match curr_state {
        TokenTypeParseState::Init
        | TokenTypeParseState::ReadingNon한글 => TokenType::No한글(no_한글s.into_iter().collect()),
        TokenTypeParseState::Reading한글 => TokenType::Only한글(into_자모s(한글s)),
    }
}
