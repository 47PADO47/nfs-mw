use super::raw::{chunks, tags, units, words};
use super::response::read_response_lists;
use super::script::read_script;
use super::types::*;
use crate::error::{Error, Result};

/// Reads one `FObj` chunk. Returns `None` for an object without an `ObjD` (nothing to show).
pub fn read_object(data: &[u8]) -> Result<Option<ObjectDef>> {
    let mut obj = ObjectDef {
        kind: ObjectKind::Group,
        guid: 0,
        name_hash: 0,
        flags: 0,
        resource: None,
        parent: None,
        data: ObjectData::default(),
        string: None,
        multi: None,
        scripts: Vec::new(),
        responses: Vec::new(),
    };
    let mut seen = false;
    for c in chunks(data, "FObj")? {
        match &c.id {
            b"ObjD" => {
                seen = true;
                read_object_data(&mut obj, c.data)?;
            }
            b"Scrp" => obj.scripts.push(read_script(c.data)?),
            b"MsgR" => obj.responses = read_response_lists(c.data, "MsgR")?,
            _ => {}
        }
    }
    Ok(seen.then_some(obj))
}

fn read_object_data(obj: &mut ObjectDef, data: &[u8]) -> Result<()> {
    let mut string = StringDef::default();
    let mut multi = MultiDef::default();
    let (mut is_string, mut is_multi) = (false, false);
    for t in tags(data, "ObjD")? {
        let value = t.u32();
        match &t.id {
            b"Ot" => obj.kind = ObjectKind::from_id(value.unwrap_or(0)),
            b"Oh" => obj.name_hash = value.unwrap_or(0),
            b"OP" => {
                if t.data.len() < 16 {
                    return Err(Error::Malformed("OP", format!("{} bytes", t.data.len())));
                }
                let word = |i: usize| u32::from_le_bytes(t.data[i * 4..i * 4 + 4].try_into().unwrap_or([0; 4]));
                obj.guid = word(0);
                obj.flags = word(2);
                let res = word(3);
                obj.resource = (res != 0xFFFF && res != 0xFFFF_FFFF).then_some(res as usize);
            }
            b"PA" => obj.parent = value.filter(|p| *p != 0),
            b"SA" => obj.data.words = words(t.data).collect(),
            b"St" => {
                is_string = true;
                let units: Vec<u16> = units(t.data).collect();
                let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
                string.text = String::from_utf16_lossy(&units[..end]);
            }
            b"SH" => {
                is_string = true;
                string.label = value.unwrap_or(0);
            }
            b"Sj" => string.justification = value.unwrap_or(0),
            b"Sl" => string.leading = t.i32().unwrap_or(0),
            b"Sw" => string.max_width = t.i32().unwrap_or(0),
            b"M1" | b"M2" | b"M3" => {
                is_multi = true;
                multi.textures[(t.id[1] - b'1') as usize] = value.unwrap_or(0);
            }
            b"Ma" | b"Mb" | b"Mc" => multi.flags[(t.id[1] - b'a') as usize] = value.unwrap_or(0),
            _ => {}
        }
    }
    if is_string || obj.kind == ObjectKind::String {
        obj.string = Some(string);
    }
    if is_multi || obj.kind == ObjectKind::MultiImage {
        obj.multi = Some(multi);
    }
    Ok(())
}
