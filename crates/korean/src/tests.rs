use crate::tokenize;

#[test]
fn tokenize_korean() {
    let sample = [
        ("배현솔", vec!["배현솔"]),
        ("배현솔이", vec!["배현솔"]),
        ("배현솔가", vec!["배현솔가"]),
        ("나는", vec!["나"]),
        ("날는", vec!["날는"]),
        ("날은", vec!["날"]),
        ("은", vec!["은"]),
        ("는", vec!["는"]),
        ("이", vec!["이"]),
        ("가", vec!["가"]),
        ("호랑이고", vec!["호랑"]),
        ("사슴이고", vec!["사슴"]),
        ("호랑이라고", vec!["호랑"]),
        ("사슴이라고", vec!["사슴"]),
        ("호랑라고", vec!["호랑라고"]),
        ("사슴라고", vec!["사슴라고"]),
        ("사슴고", vec!["사슴고"]),
        ("", vec![""]),
        ("abc가나다", vec!["abc", "가나다"]),
        ("비도", vec!["비"]),
        ("오고", vec!["오고"]),
        ("그래서", vec!["그래서"]),
        ("너의", vec!["너"]),
        ("생각이", vec!["생각"]),
        ("났어", vec!["났어"]),
        ("너랑", vec!["너"]),
        ("널랑", vec!["널랑"]),
        ("abc", vec!["abc"]),

        // TODO: it has to be `vec!["「", "형사소송법", "」"]`, but the current implementation has limitations
        ("「형사소송법」", vec!["「", "형사소송법」"]),
    ];

    for (s, answer) in sample.into_iter() {
        let tokenized = tokenize(s);
        assert_eq!(tokenized, answer);
    }
}
