use super::raw::{read_u32, tags, words};
use super::types::*;
use crate::error::Result;

/// Reads the body of a `MsgR` or `PkgR` chunk: for each message id `Mi`, its responses `Ri` with parameter
/// `Ru` or `Rs` and target `Rt`.
pub fn read_response_lists(data: &[u8], what: &'static str) -> Result<Vec<ResponseList>> {
    let mut lists: Vec<ResponseList> = Vec::new();
    for t in tags(data, what)? {
        match &t.id {
            b"Mi" => lists.push(ResponseList { message: t.u32().unwrap_or(0), responses: Vec::new() }),
            b"Ri" => {
                if let Some(list) = lists.last_mut() {
                    list.responses.push(Response {
                        kind: ResponseKind::from_id(t.u32().unwrap_or(u32::MAX)),
                        param: ResponseParam::Number(0),
                        target: 0,
                    });
                }
            }
            b"Ru" => {
                if let Some(r) = lists.last_mut().and_then(|l| l.responses.last_mut()) {
                    r.param = ResponseParam::Number(t.u32().unwrap_or(0));
                }
            }
            b"Rs" => {
                if let Some(r) = lists.last_mut().and_then(|l| l.responses.last_mut()) {
                    let end = t.data.iter().position(|b| *b == 0).unwrap_or(t.data.len());
                    r.param = ResponseParam::Text(String::from_utf8_lossy(&t.data[..end]).into_owned());
                }
            }
            b"Rt" => {
                if let Some(r) = lists.last_mut().and_then(|l| l.responses.last_mut()) {
                    r.target = t.u32().unwrap_or(0);
                }
            }
            _ => {}
        }
    }
    Ok(lists)
}

/// Reads the `Targ` chunk: `Mt` entries `{message id, target GUIDs...}`.
pub fn read_targets(data: &[u8]) -> Result<Vec<MessageTargets>> {
    let mut out = Vec::new();
    for t in tags(data, "Targ")? {
        if &t.id == b"Mt" {
            let Some(message) = read_u32(t.data, 0) else { continue };
            let guids = words(&t.data[4..]).collect();
            out.push(MessageTargets { message, guids });
        }
    }
    Ok(out)
}
