use clap::Parser;
use ragit_api::{Model, get_model_by_name, load_models};
use ragit_pdl::Pdl;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = None)]
    api_key: Option<String>,

    /// Path of an input pdl file
    #[arg(short, long)]
    input: String,

    /// Path of an output pdl file\
    /// If it's 'STDOUT', the response from LLM is dumped to stdout
    #[arg(short, long, default_value_t = String::from("STDOUT"))]
    output: String,

    /// path of `models.json`. If it's not given, it uses default models.
    #[arg(long, default_value = None)]
    models_at: Option<String>,

    /// Name of the model you want to use. Below is a list of default models.
    /// haiku | sonnet | llama-8b | llama-70b | gpt-4o | gpt-4o-mini
    #[arg(short, long, default_value_t = String::from("llama-70b"))]
    model: String,

    #[arg(long, default_value = None)]
    temperature: Option<f64>,

    #[arg(long, default_value_t = 0)]
    max_retry: usize,

    /// milliseconds
    #[arg(long, default_value_t = 5_000)]
    sleep_between_retries: u64,

    #[arg(long, default_value = None)]
    max_tokens: Option<usize>,

    /// milliseconds\
    /// If it's "d", it uses models' default timeout value (defined in this library)\
    /// If it's "n", there's no timeout\
    /// Otherwise, it calls `parse::<u64>()`
    #[arg(long, default_value_t = String::from("d"))]
    timeout: String,

    #[arg(long, default_value = None)]
    frequency_penalty: Option<f64>,

    #[arg(long, default_value_t = false)]
    strict_mode: bool,

    #[arg(long, default_value_t = false)]
    dump_json: bool,
}

// TODO: interactive ui like ollama
#[tokio::main]
async fn main() {
    let args = Args::parse();

    let Pdl { messages, schema } = ragit_pdl::parse_pdl_from_file(
        &args.input,
        &tera::Context::new(),
        args.strict_mode,

        // TODO: escape input
        false,  // is_escaped
    ).unwrap();
    let models = if let Some(models_at) = &args.models_at {
        load_models(models_at).unwrap()
    } else {
        Model::default_models()
    };
    let model = get_model_by_name(&models, &args.model).unwrap();
    let timeout = match &args.timeout {
        t if t == "d" => Some(model.api_timeout * 1000),
        t if t == "n" => None,
        t => Some(t.parse::<u64>().unwrap()),
    };

    let request = ragit_api::Request {
        messages,
        model,
        temperature: args.temperature,
        dump_pdl_at: if args.output != "STDOUT" { Some(args.output.clone()) } else { None },
        dump_json_at: if args.dump_json { Some(String::from(".")) } else { None },
        max_retry: args.max_retry,
        max_tokens: args.max_tokens,
        timeout,
        sleep_between_retries: args.sleep_between_retries,
        frequency_penalty: args.frequency_penalty,
        schema: schema.clone(),
        schema_max_try: 3,

        // TODO: make it configurable
        record_api_usage_at: None,
    };

    let response = if schema.is_none() {
        request.send().await.unwrap().get_message(0).unwrap().to_string()
    } else {
        request.send_and_validate::<serde_json::Value>(serde_json::Value::Null).await.unwrap().to_string()
    };

    if args.output == "STDOUT" {
        println!("{response}");
    }
}
