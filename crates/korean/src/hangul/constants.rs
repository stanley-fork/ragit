use lazy_static::lazy_static;

pub const 초성S: [u16; 19] = [
    'ㄱ' as u16,
    'ㄲ' as u16,
    'ㄴ' as u16,
    'ㄷ' as u16,
    'ㄸ' as u16,
    'ㄹ' as u16,
    'ㅁ' as u16,
    'ㅂ' as u16,
    'ㅃ' as u16,
    'ㅅ' as u16,
    'ㅆ' as u16,
    'ㅇ' as u16,
    'ㅈ' as u16,
    'ㅉ' as u16,
    'ㅊ' as u16,
    'ㅋ' as u16,
    'ㅌ' as u16,
    'ㅍ' as u16,
    'ㅎ' as u16,
];

pub const 중성S: [u16; 21] = [
    'ㅏ' as u16,
    'ㅐ' as u16,
    'ㅑ' as u16,
    'ㅒ' as u16,
    'ㅓ' as u16,
    'ㅔ' as u16,
    'ㅕ' as u16,
    'ㅖ' as u16,
    'ㅗ' as u16,
    'ㅘ' as u16,
    'ㅙ' as u16,
    'ㅚ' as u16,
    'ㅛ' as u16,
    'ㅜ' as u16,
    'ㅝ' as u16,
    'ㅞ' as u16,
    'ㅟ' as u16,
    'ㅠ' as u16,
    'ㅡ' as u16,
    'ㅢ' as u16,
    'ㅣ' as u16,
];

pub const 종성S: [u16; 27] = [
    'ㄱ' as u16,
    'ㄲ' as u16,
    'ㄳ' as u16,
    'ㄴ' as u16,
    'ㄵ' as u16,
    'ㄶ' as u16,
    'ㄷ' as u16,
    'ㄹ' as u16,
    'ㄺ' as u16,
    'ㄻ' as u16,
    'ㄼ' as u16,
    'ㄽ' as u16,
    'ㄾ' as u16,
    'ㄿ' as u16,
    'ㅀ' as u16,
    'ㅁ' as u16,
    'ㅂ' as u16,
    'ㅄ' as u16,
    'ㅅ' as u16,
    'ㅆ' as u16,
    'ㅇ' as u16,
    'ㅈ' as u16,
    'ㅊ' as u16,
    'ㅋ' as u16,
    'ㅌ' as u16,
    'ㅍ' as u16,
    'ㅎ' as u16,
];

lazy_static! {
    pub static ref 초성_REV: Vec<u16> = {
        let mut r = vec![0;30];

        for (ind, c) in 초성S.iter().enumerate() {
            r[*c as usize - 'ㄱ' as usize] = ind as u16;
        }

        r
    };

    pub static ref 중성_REV: Vec<u16> = {
        let mut r = vec![0;30];

        for (ind, c) in 중성S.iter().enumerate() {
            r[*c as usize - 'ㅏ' as usize] = ind as u16;
        }

        r
    };

    pub static ref 종성_REV: Vec<u16> = {
        let mut r = vec![0;30];

        for (ind, c) in 종성S.iter().enumerate() {
            r[*c as usize - 'ㄱ' as usize] = ind as u16;
        }

        r
    };
}
