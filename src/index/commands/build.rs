use super::Index;
use crate::error::Error;
use ragit_api::record::Record;

impl Index {
    /// This is `dashboard` of `rag bulid --dashboard`. It clears the screen when called.
    pub fn render_dashboard(&self) -> Result<(), Error> {
        clearscreen::clear().expect("failed to clear screen");
        println!("staged files: {}, processed files: {}", self.staged_files.len(), self.processed_files.len());
        println!("chunks: {}, chunk files: {}", self.chunk_count, self.chunk_files.len());

        if let Some(file) = &self.curr_processing_file {
            println!("curr processing file: {file}");
        }

        else {
            println!("");
        }

        println!("model: {}", self.api_config.model.to_human_friendly_name());

        let api_records = self.api_config.get_api_usage("create_chunk_from")?;
        let mut input_tokens = 0;
        let mut output_tokens = 0;
        let mut input_cost = 0;
        let mut output_cost = 0;

        for Record { input, output, input_weight, output_weight, .. } in api_records.iter() {
            input_tokens += input;
            output_tokens += output;
            input_cost += input * input_weight;
            output_cost += output * output_weight;
        }

        println!(
            "input tokens: {input_tokens} ({:.3}$), output tokens: {output_tokens} ({:.3}$)",
            input_cost as f64 / 1_000_000_000.0,
            output_cost as f64 / 1_000_000_000.0,
        );
        Ok(())
    }
}
