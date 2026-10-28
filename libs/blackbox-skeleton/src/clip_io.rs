//! Reading and writing clips in the pack.

use glam::{Quat, Vec3};

use crate::bytes::{Reader, Writer};
use crate::clip::{Channel, Clip, Notify, Track};
use crate::error::{Error, Result};

const NONE: u8 = 0;
const CONSTANT: u8 = 1;
const UNIFORM: u8 = 2;
const KEYED: u8 = 3;
const FLAG_ADDITIVE: u8 = 1;
const FLAG_ROOT_MOTION: u8 = 2;

/// A channel value with a fixed number of floats.
trait Wire: Copy {
    const BYTES: usize;
    fn put(self, w: &mut Writer);
    fn get(r: &mut Reader) -> Result<Self>;
}

impl Wire for Quat {
    const BYTES: usize = 16;

    fn put(self, w: &mut Writer) {
        w.quat(self);
    }

    fn get(r: &mut Reader) -> Result<Self> {
        r.quat()
    }
}

impl Wire for Vec3 {
    const BYTES: usize = 12;

    fn put(self, w: &mut Writer) {
        w.vec3(self);
    }

    fn get(r: &mut Reader) -> Result<Self> {
        r.vec3()
    }
}

fn put_channel<T: Wire>(w: &mut Writer, channel: &Channel<T>) {
    match channel {
        Channel::None => w.u8(NONE),
        Channel::Constant(v) => {
            w.u8(CONSTANT);
            v.put(w);
        }
        Channel::Uniform(values) => {
            w.u8(UNIFORM);
            w.count(values.len());
            values.iter().for_each(|v| v.put(w));
        }
        Channel::Keyed { times, values } => {
            w.u8(KEYED);
            let n = times.len().min(values.len());
            w.count(n);
            times[..n].iter().for_each(|&t| w.f32(t));
            values[..n].iter().for_each(|v| v.put(w));
        }
    }
}

fn get_channel<T: Wire>(r: &mut Reader) -> Result<Channel<T>> {
    Ok(match r.u8()? {
        NONE => Channel::None,
        CONSTANT => Channel::Constant(T::get(r)?),
        UNIFORM => {
            let n = r.count(T::BYTES)?;
            Channel::Uniform((0..n).map(|_| T::get(r)).collect::<Result<_>>()?)
        }
        KEYED => {
            let n = r.count(T::BYTES + 4)?;
            let times = (0..n).map(|_| r.f32()).collect::<Result<_>>()?;
            let values = (0..n).map(|_| T::get(r)).collect::<Result<_>>()?;
            Channel::Keyed { times, values }
        }
        _ => return Err(Error::BadValue("unknown channel kind")),
    })
}

pub(crate) fn put_clip(w: &mut Writer, clip: &Clip) {
    w.str(&clip.name);
    w.f32(clip.length);
    w.u8((u8::from(clip.additive) * FLAG_ADDITIVE) | (u8::from(clip.root_motion) * FLAG_ROOT_MOTION));
    w.count(clip.notifies.len());
    for notify in &clip.notifies {
        w.str(&notify.name);
        w.f32(notify.time);
    }
    w.count(clip.tracks.len());
    for track in &clip.tracks {
        w.count(track.bone);
        put_channel(w, &track.rotation);
        put_channel(w, &track.translation);
        put_channel(w, &track.scale);
    }
}

pub(crate) fn get_clip(r: &mut Reader) -> Result<Clip> {
    let name = r.str()?;
    let length = r.f32()?;
    let flags = r.u8()?;
    let notifies = (0..r.count(6)?).map(|_| Ok(Notify { name: r.str()?, time: r.f32()? })).collect::<Result<_>>()?;
    let mut tracks = Vec::new();
    for _ in 0..r.count(7)? {
        tracks.push(Track {
            bone: r.u32()? as usize,
            rotation: get_channel(r)?,
            translation: get_channel(r)?,
            scale: get_channel(r)?,
        });
    }
    Ok(Clip {
        name,
        length,
        tracks,
        additive: flags & FLAG_ADDITIVE != 0,
        root_motion: flags & FLAG_ROOT_MOTION != 0,
        notifies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_round_trips_every_channel_kind() {
        let clip = Clip {
            name: "ride".into(),
            length: 1.5,
            tracks: vec![
                Track {
                    bone: 3,
                    rotation: Channel::Uniform(vec![Quat::IDENTITY, Quat::from_rotation_y(0.4)]),
                    translation: Channel::Keyed { times: vec![0.0, 1.0], values: vec![Vec3::X, Vec3::Y] },
                    scale: Channel::Constant(Vec3::splat(2.0)),
                },
                Track { bone: 0, rotation: Channel::None, translation: Channel::None, scale: Channel::None },
            ],
            additive: true,
            root_motion: false,
            notifies: vec![Notify { name: "step".into(), time: 0.25 }],
        };
        let mut w = Writer::default();
        put_clip(&mut w, &clip);
        let mut r = Reader::new(&w.bytes);
        assert_eq!(get_clip(&mut r).unwrap(), clip);
        assert!(r.is_done());
    }

    #[test]
    fn truncated_clip_is_an_error_not_a_panic() {
        let clip = Clip {
            name: "x".into(),
            length: 1.0,
            tracks: vec![Track {
                bone: 1,
                rotation: Channel::Uniform(vec![Quat::IDENTITY; 4]),
                translation: Channel::None,
                scale: Channel::None,
            }],
            additive: false,
            root_motion: true,
            notifies: vec![],
        };
        let mut w = Writer::default();
        put_clip(&mut w, &clip);
        for cut in 0..w.bytes.len() {
            assert!(get_clip(&mut Reader::new(&w.bytes[..cut])).is_err(), "cut {cut}");
        }
    }
}
