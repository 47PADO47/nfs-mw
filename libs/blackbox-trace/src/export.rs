//! Chrome Trace Event JSON, imported directly by Perfetto. Serialization runs after capture.
use super::{Capture, Kind};
use serde_json::json;
use std::io::{self, Write};

impl Capture {
    pub fn write_json(&self, mut output: impl Write) -> io::Result<()> {
        let pid = std::process::id();
        write!(output, "{{\"displayTimeUnit\":\"ms\",\"traceEvents\":[")?;
        serde_json::to_writer(
            &mut output,
            &json!({
                "name":"process_name", "ph":"M", "pid":pid, "tid":0,
                "args":{"name":"Application performance capture"}
            }),
        )?;
        for (thread, name) in &self.threads {
            write!(output, ",")?;
            serde_json::to_writer(
                &mut output,
                &json!({
                    "name":"thread_name", "ph":"M", "pid":pid, "tid":thread, "args":{"name":name}
                }),
            )?;
        }
        for event in &self.events {
            let mut value = json!({
                "name":event.name, "pid":pid, "tid":event.thread, "ts":event.us, "cat":"application"
            });
            match &event.kind {
                Kind::Scope(duration) => {
                    value["ph"] = json!("X");
                    value["dur"] = json!(duration);
                }
                Kind::Counter(values) => {
                    value["ph"] = json!("C");
                    value["args"] = values.iter().map(|(k, v)| ((*k).to_owned(), json!(v))).collect();
                }
                Kind::Note(text) => {
                    value["ph"] = json!("i");
                    value["s"] = json!("t");
                    value["args"] = json!({"detail":text});
                }
            }
            write!(output, ",")?;
            serde_json::to_writer(&mut output, &value)?;
        }
        write!(output, "],\"metadata\":{{\"events\":{},\"droppedEvents\":{}}}}}", self.events(), self.dropped())?;
        output.flush()
    }
}
