use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlogIndex {
    pub key: String,
    pub title: String,
    pub date: String,  // yyyy-mm-dd
    pub author: String,
    pub tags: Vec<String>,
    pub file: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderableBlogIndex {
    pub month: String,  // yyyy-mm
    pub articles: Vec<BlogIndex>,
}

// `index` must be sorted by date (either ASC or DESC)
pub fn into_renderables(index: &[BlogIndex]) -> Vec<RenderableBlogIndex> {
    let mut curr_month = String::new();
    let mut curr_articles = vec![];
    let mut result = vec![];

    for i in index.iter() {
        let i_month = i.date.get(0..7).unwrap().to_string();

        if i_month != curr_month {
            if !curr_articles.is_empty() {
                result.push(RenderableBlogIndex {
                    month: curr_month.to_string(),
                    articles: curr_articles.clone(),
                });
            }

            curr_month = i_month;
            curr_articles = vec![i.clone()];
            continue;
        }

        curr_articles.push(i.clone());
    }

    if !curr_articles.is_empty() {
        result.push(RenderableBlogIndex {
            month: curr_month.to_string(),
            articles: curr_articles.clone(),
        });
    }

    result
}
