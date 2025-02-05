use chrono::DateTime;
use chrono::offset::Local;
use crate::Error;
use crate::json_type::JsonType;
use ragit_fs::{
    WriteMode,
    read_string,
    write_string,
};
use ragit_pdl::Message;
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct RecordAt {
    pub path: String,
    pub id: String,
}

// using the same type for integers makes ser/de easier
#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub time: u64,
    pub input: u64,
    pub output: u64,

    // dollars per 1 billion tokens
    pub input_weight: u64,
    pub output_weight: u64,
}

impl From<Record> for Value {
    fn from(r: Record) -> Value {
        Value::Array(vec![
            Value::from(r.time),
            Value::from(r.input),
            Value::from(r.output),
            Value::from(r.input_weight),
            Value::from(r.output_weight),
        ])
    }
}

impl TryFrom<Value> for Record {
    type Error = Error;

    fn try_from(j: Value) -> Result<Record, Error> {
        let mut result = vec![];

        match &j {
            Value::Array(arr) => {
                if arr.len() != 5 {
                    return Err(Error::WrongSchema(format!("expected an array of length 5, but got length {}", arr.len())));
                }

                for r in arr.iter() {
                    match r.as_u64() {
                        Some(n) => {
                            result.push(n);
                        },
                        None => {
                            return Err(Error::JsonTypeError {
                                expected: JsonType::U64,
                                got: r.into(),
                            });
                        },
                    }
                }

                Ok(Record {
                    time: result[0],
                    input: result[1],
                    output: result[2],
                    input_weight: result[3],
                    output_weight: result[4],
                })
            },
            _ => Err(Error::JsonTypeError {
                expected: JsonType::Array,
                got: (&j).into(),
            }),
        }
    }
}

// why do I have to impl it manually?
fn records_from_json(j: &Value) -> Result<Vec<Record>, Error> {
    match j {
        Value::Array(arr) => {
            let mut result = vec![];

            for r in arr.iter() {
                result.push(Record::try_from(r.clone())?);
            }

            Ok(result)
        },
        _ => Err(Error::JsonTypeError {
            expected: JsonType::Array,
            got: j.into(),
        }),
    }
}

#[derive(Clone)]
pub struct Tracker(pub HashMap<String, Vec<Record>>);  // user_name -> usage

impl Tracker {
    pub fn new() -> Self {
        Tracker(HashMap::new())
    }

    pub fn load_from_file(path: &str) -> Result<Self, Error> {
        let content = read_string(path)?;
        let j: Value = serde_json::from_str(&content)?;
        Tracker::try_from(&j)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Error> {
        Ok(write_string(
            path,
            &serde_json::to_string_pretty(&Value::from(self))?,
            WriteMode::CreateOrTruncate,
        )?)
    }
}

impl TryFrom<&Value> for Tracker {
    type Error = Error;

    fn try_from(v: &Value) -> Result<Tracker, Error> {
        match v {
            Value::Object(obj) => {
                let mut result = HashMap::new();

                for (k, v) in obj.iter() {
                    result.insert(k.to_string(), records_from_json(v)?);
                }

                Ok(Tracker(result))
            },
            _ => Err(Error::JsonTypeError {
                expected: JsonType::Object,
                got: v.into(),
            }),
        }
    }
}

impl From<&Tracker> for Value {
    fn from(t: &Tracker) -> Value {
        let mut result = Map::new();

        for (key, records) in t.0.iter() {
            result.insert(
                key.to_string(),
                Value::Array(records.iter().map(
                    |record| (*record).into()
                ).collect()),
            );
        }

        result.into()
    }
}

pub fn record_api_usage(
    at: &RecordAt,
    input_count: u64,
    output_count: u64,

    // dollars per 1 billion tokens
    input_weight: u64,
    output_weight: u64,
    clean_up_records: bool,
) -> Result<(), String> {
    let mut tracker = Tracker::load_from_file(&at.path).map_err(|e| format!("{e:?}"))?;
    let new_record = Record {
        time: Local::now().timestamp().max(0) as u64,
        input: input_count,
        output: output_count,
        input_weight,
        output_weight,
    };

    match tracker.0.get_mut(&at.id) {
        Some(records) => {
            records.push(new_record);

            if clean_up_records {
                // `records` is always sorted
                let mut new_records = vec![];
                let old = Local::now().timestamp().max(1 << 41) as u64 - (1 << 41);

                for record in records.iter() {
                    if record.time < old {
                        continue;
                    }

                    match new_records.last_mut() {
                        Some(Record {
                            time,
                            input,
                            output,
                            input_weight,
                            output_weight,
                        }) if *time + (1 << 27) > record.time && *input_weight == record.input_weight && *output_weight == record.output_weight => {
                            *time = (*time + record.time) >> 1;
                            *input += record.input;
                            *output += record.output;
                        },
                        _ => {
                            new_records.push(*record);
                        },
                    }
                }

                new_records.sort_by_key(|Record { time, .. }| *time);
                *records = new_records;
            }
        },
        None => {
            tracker.0.insert(at.id.clone(), vec![new_record]);
        },
    }

    tracker.save_to_file(&at.path).map_err(|e| format!("{e:?}"))?;

    Ok(())
}

pub fn get_user_usage_data_after(at: RecordAt, after: DateTime<Local>) -> Option<Vec<Record>> {
    let after = after.timestamp().max(0) as u64;

    match Tracker::load_from_file(&at.path) {
        Ok(tracker) => match tracker.0.get(&at.id) {
            Some(records) => Some(records.iter().filter(
                |Record { time, .. }| *time > after
            ).map(
                |record| record.clone()
            ).collect()),
            None => None,
        },
        _ => None,
    }
}

pub fn get_usage_data_after(path: &str, after: DateTime<Local>) -> Option<Vec<Record>> {
    let after = after.timestamp().max(0) as u64;

    match Tracker::load_from_file(path) {
        Ok(tracker) => {
            let mut result = vec![];

            for records in tracker.0.values() {
                for record in records.iter() {
                    if record.time > after {
                        result.push(record.clone());
                    }
                }
            }

            Some(result)
        },
        _ => None,
    }
}

/// It returns the cost in dollars (in a formatted string), without any currency unit.
pub fn calc_usage(records: &[Record]) -> String {
    // cost * 1B
    let mut total: u64 = records.iter().map(
        |Record {
            time: _,
            input, input_weight,
            output, output_weight,
        }| *input * *input_weight + *output * *output_weight
    ).sum();

    // cost * 1K
    total /= 1_000_000;

    format!("{:.3}", total as f64 / 1_000.0)
}

pub fn dump_pdl(
    messages: &[Message],
    response: &str,
    reasoning: &Option<String>,
    path: &str,
    metadata: String,
) -> Result<(), Error> {
    let mut markdown = vec![];

    for message in messages.iter() {
        markdown.push(format!(
            "\n\n<|{:?}|>\n\n{}",
            message.role,
            message.content.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(""),
        ));
    }

    markdown.push(format!(
        "\n\n<|Assistant|>{}\n\n{response}",
        if let Some(reasoning) = reasoning {
            format!("\n\n<|Reasoning|>\n\n{reasoning}\n\n")
        } else {
            String::new()
        },
    ));
    markdown.push(format!("{}# {metadata} #{}", '{', '}'));  // tera format

    write_string(
        path,
        &markdown.join("\n"),
        WriteMode::CreateOrTruncate,
    )?;

    Ok(())
}
