use serde::Serialize;

#[derive(Serialize)]
pub struct TopMenu {
    items: Vec<TopMenuItem>,
}

#[derive(Serialize)]
struct TopMenuItem {
    name: String,
    href: String,
}

impl TopMenu {
    pub fn new(items: Vec<(&str, &str)>) -> Self {
        TopMenu {
            items: items.iter().map(
                |(name, href)| TopMenuItem {
                    name: name.to_string(),
                    href: href.to_string(),
                },
            ).collect(),
        }
    }
}
