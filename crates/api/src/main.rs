use clap::Parser;
use std::str::FromStr;

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

    /// claude-3.5-haiku | claude-3.5-sonnet | llama3.1-8b-groq | llama3.1-70b-groq
    /// | gpt-4o | gpt-4o-mini
    #[arg(short, long, default_value_t = String::from("llama3.1-70b-groq"))]
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
}

// TODO: interactive ui like ollama
#[tokio::main]
async fn main() {
    let args = Args::parse();

    let messages = ragit_api::messages_from_file(&args.input, tera::Context::new()).unwrap();
    let model = ragit_api::ChatModel::from_str(&args.model).unwrap();
    let timeout = match &args.timeout {
        t if t == "d" => Some(model.api_timeout()),
        t if t == "n" => None,
        t => Some(t.parse::<u64>().unwrap()),
    };

    let request = ragit_api::ChatRequest {
        messages,
        model,
        temperature: args.temperature,
        api_key: args.api_key,
        dump_pdl_at: if args.output != "STDOUT" { Some(args.output.clone()) } else { None },
        max_retry: args.max_retry,
        max_tokens: args.max_tokens,
        timeout,
        sleep_between_retries: args.sleep_between_retries,
        frequency_penalty: args.frequency_penalty,

        // TODO: make it configurable
        record_api_usage_at: None,
    };

    let response = request.send().await.unwrap();

    if args.output == "STDOUT" {
        println!("{}", response.get_message(0).unwrap());
    }
}
