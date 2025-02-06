# Models

In order to add/remove/edit language models, you have to modify `.ragit/models.json`. Below is the schema of the file.

```rust
struct ModelRaw {
    /// Model name shown to user.
    /// `rag config --set model` also
    /// uses this name.
    name: String,

    /// Model name used for api requests.
    api_name: String,

    can_read_images: bool,

    /// `openai | cohere | anthropic`
    ///
    /// If you're using an openai-compatible
    /// api, set this to `openai`.
    api_provider: String,

    /// It's necessary if you're using an
    /// openai-compatible api. If it's not
    /// set, ragit uses the default url of
    /// each api provider.
    api_url: Option<String>,

    /// Dollars per 1 million input tokens.
    input_price: f64,

    /// Dollars per 1 million output tokens.
    output_price: f64,

    /// The number is in seconds.
    /// If not set, it's default to 180 seconds.
    api_timeout: Option<u64>,

    explanation: Option<String>,

    /// If you don't want to use an env var, you
    /// can hard-code your api key in this field.
    api_key: Option<String>,

    /// If you've hard-coded your api key,
    /// you don't have to set this. If neither
    /// `api_key`, nor `api_env_var` is set,
    /// it assumes that the model doesn't require
    /// an api key.
    api_env_var: Option<String>,
}
```
