//! The module bank: the header of an `.abk` file, its modules and its interface table. Layout:
//! `docs/formats/aems.md`.

use std::sync::Arc;

use crate::error::{Error, Result};

/// A reference to a Csis interface (a class the bank implements or uses).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    /// Offset of the handle this reference fills (a module's `classHandle` for the module's own class).
    pub handle_offset: usize,
    /// 0 global variable, 1 class, 2 function.
    pub kind: u8,
    pub name: String,
}

/// One module: the logic of one class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// Offset of the module header in the file.
    pub header: usize,
    pub id: u32,
    pub max_instances: i16,
    pub globals: u16,
    pub functions: u16,
    pub players: u8,
    pub class_destructor: bool,
    pub class_data: bool,
    pub class_controllers: u8,
    /// Offsets of the code and of the data (the instance image starts here) and the data size.
    pub code: usize,
    pub data: usize,
    pub data_size: usize,
    /// Offset of the destroy node from the start of an instance.
    pub destroy_offset: usize,
    /// Offsets from the start of an instance of the player nodes, then the class-controller nodes.
    pub node_offsets: Vec<usize>,
    /// The Csis class the module implements, from the interface table.
    pub class_name: Option<String>,
}

/// A parsed bank: its modules and the resident part of the file the modules run on.
#[derive(Debug, Clone)]
pub struct ModuleBank {
    image: Arc<Vec<u8>>,
    modules: Vec<Module>,
    interfaces: Vec<Interface>,
    sounds_offset: usize,
}

fn u32_at(data: &[u8], at: usize) -> Result<u32> {
    let end = at.checked_add(4).ok_or(Error::Bad("offset overflow"))?;
    let bytes = data.get(at..end).ok_or(Error::Truncated { at, needed: 4, have: data.len() })?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn u16_at(data: &[u8], at: usize) -> Result<u16> {
    let end = at.checked_add(2).ok_or(Error::Bad("offset overflow"))?;
    let bytes = data.get(at..end).ok_or(Error::Truncated { at, needed: 2, have: data.len() })?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn byte_at(data: &[u8], at: usize) -> Result<u8> {
    data.get(at).copied().ok_or(Error::Truncated { at, needed: 1, have: data.len() })
}

fn parse_interfaces(data: &[u8], at: usize) -> Result<Vec<Interface>> {
    let count = u32_at(data, at)? as usize;
    if count > data.len() / 12 {
        return Err(Error::Bad("interface count"));
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let entry = at + 4 + 12 * i;
        let (handle, id) = (u32_at(data, entry)? as usize, u32_at(data, entry + 4)? as usize);
        let kind = byte_at(data, entry + 8)?;
        let name_at = id.checked_add(4).ok_or(Error::Bad("interface id"))?;
        let tail = data.get(name_at..).ok_or(Error::Bad("interface id outside the file"))?;
        let len = tail.iter().position(|&b| b == 0).ok_or(Error::Bad("unterminated interface name"))?;
        out.push(Interface { handle_offset: handle, kind, name: String::from_utf8_lossy(&tail[..len]).into_owned() });
    }
    Ok(out)
}

fn parse_module(data: &[u8], at: usize, resident: usize, interfaces: &[Interface]) -> Result<(Module, usize)> {
    let players = byte_at(data, at + 0x24)?;
    let controllers = byte_at(data, at + 0x27)?;
    let nodes = usize::from(players) + usize::from(controllers);
    let mut node_offsets = Vec::with_capacity(nodes);
    for i in 0..nodes {
        node_offsets.push(u32_at(data, at + 0x3C + 4 * i)? as usize);
    }
    let (code, dat) = (u32_at(data, at + 0x28)? as usize, u32_at(data, at + 0x2C)? as usize);
    let data_size = u32_at(data, at + 0x30)? as usize;
    if code >= dat || dat.checked_add(data_size).is_none_or(|end| end > resident) {
        return Err(Error::Bad("module code or data outside the resident part"));
    }
    let class_name = interfaces.iter().find(|i| i.kind == 1 && i.handle_offset == at + 4).map(|i| i.name.clone());
    let module = Module {
        header: at,
        id: u32_at(data, at)?,
        max_instances: u16_at(data, at + 0x1E)? as i16,
        globals: u16_at(data, at + 0x20)?,
        functions: u16_at(data, at + 0x22)?,
        players,
        class_destructor: byte_at(data, at + 0x25)? != 0,
        class_data: byte_at(data, at + 0x26)? != 0,
        class_controllers: controllers,
        code,
        data: dat,
        data_size,
        destroy_offset: u32_at(data, at + 0x34)? as usize,
        node_offsets,
        class_name,
    };
    Ok((module, at + 0x3C + 4 * nodes))
}

impl ModuleBank {
    /// Parses the bytes of an `.abk` file.
    pub fn parse(data: &[u8]) -> Result<ModuleBank> {
        if data.get(..4) != Some(b"ABKC") {
            return Err(Error::BadMagic);
        }
        let count = usize::from(u16_at(data, 0x0A)?);
        let resident = u32_at(data, 0x18)? as usize;
        let first = u32_at(data, 0x1C)? as usize;
        let sounds_offset = u32_at(data, 0x20)? as usize;
        if resident > data.len() || resident < first {
            return Err(Error::Bad("resident size"));
        }
        let interfaces = parse_interfaces(data, u32_at(data, 0x38)? as usize)?;
        let mut modules = Vec::with_capacity(count);
        let mut at = first;
        for _ in 0..count {
            let (module, next) = parse_module(data, at, resident, &interfaces)?;
            modules.push(module);
            at = next;
        }
        Ok(ModuleBank { image: Arc::new(data[..resident].to_vec()), modules, interfaces, sounds_offset })
    }

    pub fn modules(&self) -> &[Module] {
        &self.modules
    }

    pub fn interfaces(&self) -> &[Interface] {
        &self.interfaces
    }

    /// The first module that implements the class `name`.
    pub fn module_of(&self, name: &str) -> Option<usize> {
        self.modules.iter().position(|m| m.class_name.as_deref() == Some(name))
    }

    /// Where the sounds (`BNKl`) start in the file.
    pub fn sounds_offset(&self) -> usize {
        self.sounds_offset
    }

    pub(crate) fn image(&self) -> &Arc<Vec<u8>> {
        &self.image
    }
}
