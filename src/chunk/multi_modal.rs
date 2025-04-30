use crate::uid::Uid;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Ragit is multi-modal: a chunk may contain texts and images (and maybe more types later).
/// Ragit internally uses pdl format to handle multi-modal contents. This struct is for an interface
/// who wants to render contents of chunks. When you load a chunk from storage, it has `.data` field
/// and the field is just a string where images are encoded in a special way. You have to use
/// `into_multi_modal_contents` function to get this struct.
///
/// `rag cat-file <UID> --json` also uses this schema.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum MultiModalContent {
    Text { content: String },
    Image { uid: String },
}

/// See `MultiModalContent` struct's document.
pub fn into_multi_modal_contents(data: &str, images: &[Uid]) -> Vec<MultiModalContent> {
    let bytes = data.as_bytes();
    let images = images.iter().map(|uid| uid.to_string().into_bytes()).collect::<HashSet<_>>();
    let mut index = 0;
    let mut last_index = 0;
    let mut result = vec![];

    while index < bytes.len() {
        match bytes[index] {
            b'i' => match (bytes.get(index + 1), bytes.get(index + 2), bytes.get(index + 3)) {
                (Some(b'm'), Some(b'g'), Some(b'_')) => match bytes.get(index + 67) {
                    Some(b'0'..=b'9' | b'a'..=b'f') => {
                        if images.contains(&bytes[(index + 4)..(index + 68)]) {
                            if last_index < index {
                                result.push(MultiModalContent::Text { content: String::from_utf8(bytes[last_index..index].to_vec()).unwrap() });
                            }

                            result.push(MultiModalContent::Image { uid: String::from_utf8(bytes[(index + 4)..(index + 68)].to_vec()).unwrap() });
                            last_index = index + 68;
                            index += 68;
                        }

                        else {
                            index += 4;
                        }
                    },
                    _ => {
                        index += 4;
                    },
                },
                _ => { index += 1; },
            },
            _ => { index += 1; },
        }
    }

    if last_index < bytes.len() {
        result.push(MultiModalContent::Text { content: String::from_utf8(bytes[last_index..].to_vec()).unwrap() });
    }

    result
}

#[test]
fn into_multi_modal_contents_test() {
    let uid1 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0000000200000000";
    let uid2 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb0000000200000000";
    let fake_uid = "cccccccccccccccccccccccccccccccccccccccccccccccc0000000200000000";
    let images = vec![
        uid1.parse::<Uid>().unwrap(),
        uid2.parse::<Uid>().unwrap(),
    ];

    assert_eq!(
        into_multi_modal_contents("", &images),
        vec![],
    );
    assert_eq!(
        into_multi_modal_contents("Hi, my name is baehyunsol", &images),
        vec![MultiModalContent::Text { content: String::from("Hi, my name is baehyunsol") }],
    );
    assert_eq!(
        into_multi_modal_contents(&format!("img_{uid1}"), &images),
        vec![MultiModalContent::Image { uid: String::from(uid1) }],
    );
    assert_eq!(
        into_multi_modal_contents(&format!("img_{uid1}img_{uid1}"), &images),
        vec![MultiModalContent::Image { uid: String::from(uid1) }, MultiModalContent::Image { uid: String::from(uid1) }],
    );
    assert_eq!(
        into_multi_modal_contents(&format!("aaimg_{uid1}img_{uid1}aa"), &images),
        vec![MultiModalContent::Text { content: String::from("aa") }, MultiModalContent::Image { uid: String::from(uid1) }, MultiModalContent::Image { uid: String::from(uid1) }, MultiModalContent::Text { content: String::from("aa") }],
    );
    assert_eq!(
        into_multi_modal_contents(&format!("img_{uid1}imgimg_{uid1}"), &images),
        vec![MultiModalContent::Image { uid: String::from(uid1) }, MultiModalContent::Text { content: String::from("img") }, MultiModalContent::Image { uid: String::from(uid1) }],
    );
    assert_eq!(
        into_multi_modal_contents(&format!("Hi, my name is baehyunsolimg_{uid1}"), &images),
        vec![MultiModalContent::Text { content: String::from("Hi, my name is baehyunsol") }, MultiModalContent::Image { uid: String::from(uid1) }],
    );
    assert_eq!(
        into_multi_modal_contents(&format!("img_{uid1}Hi, my name is baehyunsol"), &images),
        vec![MultiModalContent::Image { uid: String::from(uid1) }, MultiModalContent::Text { content: String::from("Hi, my name is baehyunsol") }],
    );
    assert_eq!(
        into_multi_modal_contents(&format!("img_{uid1}Hi, my name is baehyunsolimg_{uid1}"), &images),
        vec![MultiModalContent::Image { uid: String::from(uid1) }, MultiModalContent::Text { content: String::from("Hi, my name is baehyunsol") }, MultiModalContent::Image { uid: String::from("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0000000200000000") }],
    );
    assert_eq!(
        into_multi_modal_contents(&format!("img_{fake_uid}"), &images),
        vec![MultiModalContent::Text { content: format!("img_{fake_uid}") }],
    );
    assert_eq!(
        into_multi_modal_contents(&format!("img_{uid1}img_{fake_uid}img_{uid2}"), &images),
        vec![MultiModalContent::Image { uid: String::from(uid1) }, MultiModalContent::Text { content: format!("img_{fake_uid}") }, MultiModalContent::Image { uid: String::from(uid2) }],
    );
}
