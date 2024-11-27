use super::{
    종성_REV,
    종성s,
    중성_REV,
    중성s,
    초성_REV,
    초성s,
};

pub enum 자모 {
    초성(u16),
    중성(u16),
    종성(u16),
}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct 한글 {
    pub 초성: u16,
    pub 중성: u16,
    pub 종성: Option<u16>,
}

impl 한글 {
    pub fn from_char(c: char) -> 한글 {
        한글::from_u16(c as u16)
    }

    pub fn to_char(&self) -> char {
        char::from_u32(self.to_u16() as u32).unwrap()
    }

    pub fn from_u16(c: u16) -> 한글 {
        let 초성 = ((c - 44032) / 588) as usize;
        let 중성 = ((c - 44032) % 588 / 28) as usize;
        let 종성 = ((c - 44032) % 588 % 28) as usize;

        한글 {
            초성: 초성s[초성],
            중성: 중성s[중성],
            종성: if 종성 == 0 { None } else { Some(종성s[종성 - 1]) },
        }
    }

    pub fn to_u16(&self) -> u16 {
        44032
        + 초성_rev(self.초성) * 588
        + 중성_rev(self.중성) * 28
        + 종성_rev(self.종성)
    }
}

// 'ㄱ' -> 0, 'ㄲ' -> 1, 'ㄴ' -> 2, ...
fn 초성_rev(c: u16) -> u16 {
    초성_REV[c as usize - 'ㄱ' as usize]
}

// 'ㅏ' -> 0, 'ㅑ' -> 1, ...
fn 중성_rev(c: u16) -> u16 {
    중성_REV[c as usize - 'ㅏ' as usize]
}

// None -> 0, Some('ㄱ') -> 1, Some('ㄲ') -> 2, ...
fn 종성_rev(c: Option<u16>) -> u16 {
    match c {
        None => 0,
        Some(c) => 종성_REV[c as usize - 'ㄱ' as usize] + 1,
    }
}
