//! [`ModSlot`]: one mod slot's primitives, built from the base components of `blackbox-ui`.
//!
//! Each primitive keeps its colour, font and texture in the slot's own skin, under keys made for the slot. A primitive
//! never reads the game's skin, so a mod cannot disturb another element's look. Font and texture names are resource names
//! without extension, hashed the way the game's packages name them (`blackbox_feng::resource_handle`).
//!
//! The rectangles are in FEng units with the origin at the centre of the screen. Each primitive is laid out at its own
//! rectangle, whatever bounds the slot is given. `layout` must run before `build`, as for every component.

use blackbox_feng::resource_handle;
use blackbox_modding::host::{UiPrim, UiRect};
use blackbox_ui::{Component, DrawList, Image, LayoutCx, Panel, Rect, Skin, Text};

/// A slot's components, each with the rectangle it is laid out at, and the skin they read from.
pub struct ModSlot {
    skin: Skin,
    parts: Vec<(Rect, Box<dyn Component>)>,
}

/// A value the slot's skin holds for one of its primitives.
enum Value {
    Colour(String, [u8; 4]),
    Font(String, u32),
    Texture(String, u32),
}

impl ModSlot {
    /// The slot for `prims`, drawn in order (the first is at the back).
    pub fn new(prims: &[UiPrim]) -> Self {
        let mut values = Vec::new();
        let parts: Vec<(Rect, Box<dyn Component>)> =
            prims.iter().enumerate().map(|(index, prim)| part(index, prim, &mut values)).collect();
        let skin = values.into_iter().fold(Skin::new(), |skin, value| match value {
            Value::Colour(key, rgba) => skin.with_colour(key, rgba),
            Value::Font(key, font) => skin.with_font(key, font),
            Value::Texture(key, texture) => skin.with_texture(key, texture),
        });
        Self { skin, parts }
    }
}

impl Component for ModSlot {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let inner = LayoutCx::new(&self.skin, cx.time_s);
        for (rect, part) in &mut self.parts {
            part.layout(&inner, *rect);
        }
    }

    fn build(&self, out: &mut DrawList) {
        for (_, part) in &self.parts {
            part.build(out);
        }
    }
}

/// The skin key of primitive `index`'s `kind` (`colour`, `font` or `texture`).
fn key(kind: &str, index: usize) -> String {
    format!("mod.{kind}.{index}")
}

/// The rectangle a primitive names, as a `blackbox-ui` rectangle.
fn rect_of(rect: &UiRect) -> Rect {
    Rect::new(rect.x, rect.y, rect.w, rect.h)
}

/// The component for primitive `index`, and the skin values it needs pushed onto `values`.
fn part(index: usize, prim: &UiPrim, values: &mut Vec<Value>) -> (Rect, Box<dyn Component>) {
    match prim {
        UiPrim::Panel { rect, colour, texture } => {
            let colour_key = key("colour", index);
            values.push(Value::Colour(colour_key.clone(), *colour));
            let mut panel = Panel::new(colour_key);
            if let Some(name) = texture {
                let texture_key = key("texture", index);
                values.push(Value::Texture(texture_key.clone(), resource_handle(name)));
                panel = panel.textured(texture_key);
            }
            (rect_of(rect), Box::new(panel))
        }
        UiPrim::Text { rect, colour, justify, font, text } => {
            let colour_key = key("colour", index);
            values.push(Value::Colour(colour_key.clone(), *colour));
            let font_key = key("font", index);
            // An empty font name leaves the key unset, so the text uses font 0 (the default), as the skin rules say.
            if !font.is_empty() {
                values.push(Value::Font(font_key.clone(), resource_handle(font)));
            }
            let component = Text::new(font_key, text.clone()).with_colour_key(colour_key).with_justification(*justify);
            (rect_of(rect), Box::new(component))
        }
        UiPrim::Image { rect, uv, texture } => {
            let texture_key = key("texture", index);
            values.push(Value::Texture(texture_key.clone(), resource_handle(texture)));
            (rect_of(rect), Box::new(Image::new(texture_key).with_uv(*uv)))
        }
    }
}
