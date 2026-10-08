use glam::{Quat, Vec3};

/// What an object is, from the `Ot` tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectKind {
    Group,
    Image,
    String,
    MultiImage,
    ColoredImage,
    SimpleImage,
    Movie,
    /// A type this reader does not interpret (lists, animated images, ...).
    Other(u32),
}

impl ObjectKind {
    pub fn from_id(id: u32) -> Self {
        match id {
            1 => Self::Image,
            2 => Self::String,
            5 => Self::Group,
            7 => Self::Movie,
            9 => Self::ColoredImage,
            11 => Self::SimpleImage,
            12 => Self::MultiImage,
            other => Self::Other(other),
        }
    }

    /// True for the kinds that draw a textured quad.
    pub fn is_image(self) -> bool {
        matches!(self, Self::Image | Self::MultiImage | Self::ColoredImage | Self::SimpleImage)
    }
}

/// Engine bits of [`ObjectDef::flags`].
pub mod flags {
    /// The object takes pad focus.
    pub const IS_BUTTON: u32 = 1 << 28;
    /// A button the focus never moves to.
    pub const IGNORE_BUTTON: u32 = 1 << 26;
    /// The focus does not move away from this button by geometry.
    pub const DONT_NAVIGATE: u32 = 1 << 19;
}

/// What a resource request names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    Image,
    Font,
    MultiImage,
    Other(u32),
}

/// One entry of the package resource list: a texture or a font by file name.
#[derive(Clone, Debug)]
pub struct Resource {
    pub name: String,
    pub kind: ResourceKind,
    pub flags: u32,
    /// `hash(UPPER(base name without extension))`: the key of the texture or font at run time.
    pub handle: u32,
}

/// The data block (`SA`) of an object as 32-bit words, laid out like the file:
/// colour 0..4 (blue, green, red, alpha as i32), pivot 4..7, position 7..10, rotation 10..14 (x, y, z, w),
/// size 14..17, then the kind-specific tail (image UVs from word 17). Scripts address it by word offset.
#[derive(Clone, Debug, Default)]
pub struct ObjectData {
    pub words: Vec<u32>,
}

/// Word offsets into [`ObjectData::words`].
pub mod word {
    pub const COLOUR: usize = 0;
    pub const PIVOT: usize = 4;
    pub const POSITION: usize = 7;
    pub const ROTATION: usize = 10;
    pub const SIZE: usize = 14;
    pub const UV: usize = 17;
}

impl ObjectData {
    pub fn f32_at(&self, word: usize) -> f32 {
        self.words.get(word).copied().map_or(0.0, f32::from_bits)
    }

    pub fn i32_at(&self, word: usize) -> i32 {
        self.words.get(word).copied().unwrap_or(0) as i32
    }

    pub fn set_f32(&mut self, word: usize, v: f32) {
        if let Some(w) = self.words.get_mut(word) {
            *w = v.to_bits();
        }
    }

    pub fn set_i32(&mut self, word: usize, v: i32) {
        if let Some(w) = self.words.get_mut(word) {
            *w = v as u32;
        }
    }

    fn vec3(&self, word: usize) -> Vec3 {
        Vec3::new(self.f32_at(word), self.f32_at(word + 1), self.f32_at(word + 2))
    }

    /// Colour as `[red, green, blue, alpha]`, each clamped to 0..255.
    pub fn colour_rgba(&self) -> [u8; 4] {
        let c = |i: usize| self.i32_at(word::COLOUR + i).clamp(0, 255) as u8;
        [c(2), c(1), c(0), c(3)]
    }

    pub fn set_alpha(&mut self, alpha: i32) {
        self.set_i32(word::COLOUR + 3, alpha);
    }

    pub fn alpha(&self) -> i32 {
        self.i32_at(word::COLOUR + 3)
    }

    pub fn pivot(&self) -> Vec3 {
        self.vec3(word::PIVOT)
    }

    pub fn position(&self) -> Vec3 {
        self.vec3(word::POSITION)
    }

    pub fn size(&self) -> Vec3 {
        self.vec3(word::SIZE)
    }

    pub fn rotation(&self) -> Quat {
        let q = Quat::from_xyzw(
            self.f32_at(word::ROTATION),
            self.f32_at(word::ROTATION + 1),
            self.f32_at(word::ROTATION + 2),
            self.f32_at(word::ROTATION + 3),
        );
        if q.length_squared() > 1e-12 { q.normalize() } else { Quat::IDENTITY }
    }

    pub fn set_rotation(&mut self, q: Quat) {
        for (i, v) in [q.x, q.y, q.z, q.w].into_iter().enumerate() {
            self.set_f32(word::ROTATION + i, v);
        }
    }

    pub fn set_position(&mut self, p: Vec3) {
        for (i, v) in [p.x, p.y, p.z].into_iter().enumerate() {
            self.set_f32(word::POSITION + i, v);
        }
    }

    /// UV rectangle `[u0, v0, u1, v1]` (upper-left, lower-right) of an image; the unit square if absent.
    pub fn uv(&self) -> [f32; 4] {
        if self.words.len() >= word::UV + 4 {
            [self.f32_at(word::UV), self.f32_at(word::UV + 1), self.f32_at(word::UV + 2), self.f32_at(word::UV + 3)]
        } else {
            [0.0, 0.0, 1.0, 1.0]
        }
    }
}

/// The text of a string object.
#[derive(Clone, Debug, Default)]
pub struct StringDef {
    pub text: String,
    /// Language table key; 0 when the string has none.
    pub label: u32,
    pub justification: u32,
    pub leading: i32,
    pub max_width: i32,
}

/// The extra texture slots of a multi image.
#[derive(Clone, Copy, Debug, Default)]
pub struct MultiDef {
    pub textures: [u32; 3],
    pub flags: [u32; 3],
}

/// One animation script of an object.
#[derive(Clone, Debug, Default)]
pub struct Script {
    pub id: u32,
    pub length: i32,
    pub flags: u32,
    pub chain: Option<u32>,
    pub tracks: Vec<Track>,
    pub events: Vec<Event>,
}

impl Script {
    /// 0 once, 1 loop, 2 ping-pong.
    pub fn end_behaviour(&self) -> u32 {
        self.flags & 3
    }
}

/// What a track animates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamType {
    Int,
    Float,
    Vec2,
    Vec3,
    Quat,
    Colour,
}

impl ParamType {
    pub fn from_id(id: u8) -> Option<Self> {
        Some(match id {
            1 => Self::Int,
            2 => Self::Float,
            3 => Self::Vec2,
            4 => Self::Vec3,
            5 => Self::Quat,
            6 => Self::Colour,
            _ => return None,
        })
    }

    /// Number of 32-bit components.
    pub fn components(self) -> usize {
        match self {
            Self::Int | Self::Float => 1,
            Self::Vec2 => 2,
            Self::Vec3 => 3,
            Self::Quat | Self::Colour => 4,
        }
    }

    /// True when the components are integers (rounded when interpolated).
    pub fn is_integer(self) -> bool {
        matches!(self, Self::Int | Self::Colour)
    }
}

/// How a track blends between keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interp {
    /// Steps.
    None,
    Linear,
    /// Linear, and the first key is rewritten to the current value when the script starts.
    MoveTo,
    /// Types the game does not implement: the value is left alone.
    Unsupported,
}

/// One key: a time in ticks and up to four value components (raw words, see [`ParamType`]).
#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub time: i32,
    pub value: [u32; 4],
}

/// One animated property of a script.
#[derive(Clone, Debug)]
pub struct Track {
    pub param: ParamType,
    pub interp: Interp,
    /// Low seven bits: how the track behaves at its end (0 clamp, 1 loop, 2 ping-pong).
    pub action: u8,
    pub length: i32,
    /// Offset in 32-bit words into the object data block.
    pub offset: usize,
    /// The first key (time -1): the base the others are added to.
    pub base: Key,
    pub keys: Vec<Key>,
}

/// A message a script sends at a time.
#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub message: u32,
    /// 0 = every package, a GUID = that object, `0xFFFFFFFF` = the game, other special values.
    pub target: u32,
    pub time: u32,
}

/// What a response does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseKind {
    SetScript,
    PostToFEng,
    PostToGame,
    PostToSound,
    /// `0x100..=0x1FF`: button control (the id is kept).
    Button(u32),
    /// `0x200..=0x2FF`: package control (the id is kept).
    Package(u32),
    IfScriptEquals,
    IfScriptNotEquals,
    Else,
    EndIf,
    Other(u32),
}

impl ResponseKind {
    pub fn from_id(id: u32) -> Self {
        match id {
            0 => Self::SetScript,
            1 => Self::PostToFEng,
            2 => Self::PostToGame,
            3 => Self::PostToSound,
            0x100..=0x1FF => Self::Button(id),
            0x200..=0x2FF => Self::Package(id),
            0x300 => Self::IfScriptEquals,
            0x301 => Self::IfScriptNotEquals,
            0x500 => Self::Else,
            0x501 => Self::EndIf,
            other => Self::Other(other),
        }
    }
}

/// The parameter of a response: a number (a message or script id, a GUID) or a string (a package name).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseParam {
    Number(u32),
    Text(String),
}

impl ResponseParam {
    pub fn number(&self) -> u32 {
        match self {
            Self::Number(n) => *n,
            Self::Text(_) => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Response {
    pub kind: ResponseKind,
    pub param: ResponseParam,
    /// 0 = the object itself; a GUID; `0xFFFFFFFF` the game.
    pub target: u32,
}

/// The responses to one message id.
#[derive(Clone, Debug)]
pub struct ResponseList {
    pub message: u32,
    pub responses: Vec<Response>,
}

/// A message id and the objects it is sent to (`Targ`).
#[derive(Clone, Debug)]
pub struct MessageTargets {
    pub message: u32,
    pub guids: Vec<u32>,
}

/// One object of a package, as stored.
#[derive(Clone, Debug)]
pub struct ObjectDef {
    pub kind: ObjectKind,
    pub guid: u32,
    pub name_hash: u32,
    pub flags: u32,
    /// Index into [`Package::resources`](super::Package::resources).
    pub resource: Option<usize>,
    /// GUID of the parent group; `None` for top-level objects.
    pub parent: Option<u32>,
    pub data: ObjectData,
    pub string: Option<StringDef>,
    pub multi: Option<MultiDef>,
    pub scripts: Vec<Script>,
    pub responses: Vec<ResponseList>,
}
