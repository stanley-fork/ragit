use serde::Serialize;

#[derive(Serialize)]
pub struct SortCategory {
    pub long: String,
    pub short: String,
    pub selected: bool,
    pub extra_query_string: String,
}
