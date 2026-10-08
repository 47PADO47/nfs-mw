//! Builds package bytes for tests, mirroring the layout in `docs/formats/frontend.md`.

pub fn chunk(id: &[u8; 4], nested: bool, data: &[u8]) -> Vec<u8> {
    let mut out = id.to_vec();
    if nested {
        out[3] |= 0x80;
    }
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    out
}

pub fn tag(id: &[u8; 2], data: &[u8]) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend_from_slice(&(data.len() as u16).to_le_bytes());
    out.extend_from_slice(data);
    out
}

pub fn u32s(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

pub fn f32s(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// An object under construction.
pub struct Obj {
    pub kind: u32,
    pub guid: u32,
    pub name: u32,
    pub parent: u32,
    pub resource: u32,
    pub flags: u32,
    pub colour: [i32; 4],
    pub position: [f32; 3],
    pub size: [f32; 3],
    pub text: Option<String>,
    pub label: u32,
    /// First extra texture of a multi image (the mask); a multi image has the rotation block in its data.
    pub mask: u32,
    pub scripts: Vec<Vec<u8>>,
    pub responses: Vec<u8>,
}

impl Obj {
    pub fn new(kind: u32, guid: u32, name: u32) -> Self {
        Self {
            kind,
            guid,
            name,
            parent: 0,
            resource: 0xFFFF,
            flags: 0,
            colour: [255, 255, 255, 255],
            position: [0.0; 3],
            size: [1.0; 3],
            text: None,
            label: 0,
            mask: 0,
            scripts: Vec::new(),
            responses: Vec::new(),
        }
    }

    pub fn group(guid: u32, name: u32) -> Self {
        Self::new(5, guid, name)
    }

    pub fn image(guid: u32, name: u32) -> Self {
        let mut o = Self::new(1, guid, name);
        o.size = [10.0, 10.0, 1.0];
        o
    }

    /// A multi image (type 12) with `mask` as its second texture.
    pub fn multi(guid: u32, name: u32, mask: u32) -> Self {
        let mut o = Self::new(12, guid, name);
        o.size = [10.0, 10.0, 1.0];
        o.mask = mask;
        o
    }

    pub fn string(guid: u32, name: u32, text: &str) -> Self {
        let mut o = Self::new(2, guid, name);
        o.text = Some(text.to_string());
        o
    }

    pub fn build(&self) -> Vec<u8> {
        let mut sa = Vec::new();
        for c in self.colour {
            sa.extend_from_slice(&c.to_le_bytes());
        }
        sa.extend(f32s(&[0.0, 0.0, 0.0]));
        sa.extend(f32s(&self.position));
        sa.extend(f32s(&[0.0, 0.0, 0.0, 1.0]));
        sa.extend(f32s(&self.size));
        if self.kind == 1 || self.kind == 12 {
            sa.extend(f32s(&[0.0, 0.0, 1.0, 1.0]));
        }
        if self.kind == 12 {
            // Upper-left and lower-right UVs of the three textures, then the mask pivot and rotation.
            sa.extend(f32s(&[0.0; 6]));
            sa.extend(f32s(&[1.0; 6]));
            sa.extend(f32s(&[0.5, 0.5, 0.0]));
        }
        let mut objd = Vec::new();
        objd.extend(tag(b"Ot", &u32s(&[self.kind])));
        objd.extend(tag(b"Oh", &u32s(&[self.name])));
        objd.extend(tag(b"OP", &u32s(&[self.guid, self.name, self.flags, self.resource])));
        if self.parent != 0 {
            objd.extend(tag(b"PA", &u32s(&[self.parent])));
        }
        objd.extend(tag(b"SA", &sa));
        if self.kind == 12 {
            objd.extend(tag(b"M1", &u32s(&[self.mask])));
        }
        if let Some(text) = &self.text {
            let utf16: Vec<u8> = text.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
            objd.extend(tag(b"St", &utf16));
            objd.extend(tag(b"SH", &u32s(&[self.label])));
            objd.extend(tag(b"Sj", &u32s(&[0])));
            objd.extend(tag(b"Sl", &u32s(&[0])));
            objd.extend(tag(b"Sw", &u32s(&[0])));
        }
        let mut body = chunk(b"ObjD", true, &objd);
        for s in &self.scripts {
            body.extend(chunk(b"Scrp", true, s));
        }
        if !self.responses.is_empty() {
            body.extend(chunk(b"MsgR", true, &self.responses));
        }
        chunk(b"FObj", true, &body)
    }
}

/// A script: id, length, flags, optional chain, tracks `(param type, interp, action, word offset, keys)`,
/// events.
pub struct ScriptSpec {
    pub id: u32,
    pub length: i32,
    pub flags: u32,
    pub chain: Option<u32>,
    pub tracks: Vec<TrackSpec>,
    pub events: Vec<(u32, u32, u32)>,
}

pub struct TrackSpec {
    pub param: u8,
    pub interp: u8,
    pub action: u8,
    pub offset: u32,
    pub length: u32,
    /// `(time, words)`, the first with time -1.
    pub keys: Vec<(i32, Vec<u32>)>,
}

impl ScriptSpec {
    pub fn build(&self) -> Vec<u8> {
        let mut out = tag(b"Sh", &u32s(&[self.id, self.length as u32, self.flags, self.tracks.len() as u32]));
        if let Some(c) = self.chain {
            out.extend(tag(b"Sc", &u32s(&[c])));
        }
        for t in &self.tracks {
            let size = t.keys.first().map_or(4, |k| k.1.len() * 4) as u8;
            let mut fi = vec![t.param, size, t.interp, t.action];
            fi.extend_from_slice(&t.length.to_le_bytes());
            out.extend(tag(b"FI", &fi));
            out.extend(tag(b"To", &u32s(&[t.offset])));
            let mut kd = Vec::new();
            for (time, words) in &t.keys {
                kd.extend_from_slice(&time.to_le_bytes());
                kd.extend(u32s(words));
            }
            out.extend(tag(b"Kd", &kd));
        }
        if !self.events.is_empty() {
            let ev: Vec<u32> = self.events.iter().flat_map(|e| [e.0, e.1, e.2]).collect();
            out.extend(tag(b"EV", &u32s(&ev)));
        }
        out
    }
}

/// One response: `(response id, parameter, target)`.
pub type Resp = (u32, u32, u32);

/// `MsgR` / `PkgR` body: `(message, responses)`.
pub fn responses(list: &[(u32, Vec<Resp>)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (msg, rs) in list {
        out.extend(tag(b"Mi", &u32s(&[*msg])));
        out.extend(tag(b"MC", &u32s(&[rs.len() as u32])));
        for (id, param, target) in rs {
            out.extend(tag(b"Ri", &u32s(&[*id])));
            out.extend(tag(b"Ru", &u32s(&[*param])));
            out.extend(tag(b"Rt", &u32s(&[*target])));
        }
    }
    out
}

/// A whole package payload (the contents of an FEngPackage chunk).
pub fn package(
    name: &str,
    resources: &[(&str, u32)],
    objects: &[Obj],
    pkg_responses: &[u8],
    targets: &[(u32, Vec<u32>)],
) -> Vec<u8> {
    let long = format!("Global\\{name}");
    let mut hd =
        u32s(&[0x20000, 0, resources.len() as u32, objects.len() as u32, name.len() as u32 + 1, long.len() as u32 + 1]);
    hd.extend_from_slice(name.as_bytes());
    hd.push(0);
    hd.extend_from_slice(long.as_bytes());
    hd.push(0);

    let mut names = Vec::new();
    let mut rq = u32s(&[resources.len() as u32]);
    for (i, (n, kind)) in resources.iter().enumerate() {
        rq.extend(u32s(&[i as u32, names.len() as u32, *kind, 0, 0, 0]));
        names.extend_from_slice(n.as_bytes());
        names.push(0);
    }
    let resl = [chunk(b"RsNm", false, &names), chunk(b"RsRq", false, &rq)].concat();

    let mut objl = chunk(b"Butn", false, &u32s(&[0]));
    for o in objects {
        objl.extend(o.build());
    }
    let mut targ = tag(b"Tc", &u32s(&[targets.len() as u32]));
    for (m, g) in targets {
        let mut v = vec![*m];
        v.extend(g);
        targ.extend(tag(b"Mt", &u32s(&v)));
    }
    let mut body = chunk(b"PkHd", false, &hd);
    body.extend(chunk(b"TypS", false, &u32s(&[5, 68])));
    body.extend(chunk(b"ResL", true, &resl));
    body.extend(chunk(b"ObjL", true, &objl));
    body.extend(chunk(b"Targ", false, &targ));
    if !pkg_responses.is_empty() {
        body.extend(chunk(b"PkgR", false, pkg_responses));
    }
    chunk(b"FEng", true, &body)
}
