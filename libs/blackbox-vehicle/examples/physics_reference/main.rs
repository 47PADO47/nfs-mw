//! Numeric stdin/stdout adapter for independent, private reference harnesses.
//! Contract: `docs/specs/physics-reference-driver.md`. Contains no reference data.

use std::error::Error;
use std::io::{self, BufRead, Write};

use blackbox_vehicle::rigid_body::{
    ContactMoment, FrictionState, Obb, PlaneContact, box_contact, inverse_world_inertia, react_moments,
    transform_vector,
};
use glam::{Mat3, Vec3};

struct Input<'a>(std::str::SplitWhitespace<'a>);

impl Input<'_> {
    fn token(&mut self) -> Result<&str, Box<dyn Error>> {
        self.0.next().ok_or_else(|| "missing input field".into())
    }

    fn number(&mut self) -> Result<f32, Box<dyn Error>> {
        let value: f32 = self.token()?.parse()?;
        if !value.is_finite() {
            return Err("nonfinite input field".into());
        }
        Ok(value)
    }

    fn vector(&mut self) -> Result<Vec3, Box<dyn Error>> {
        Ok(Vec3::new(self.number()?, self.number()?, self.number()?))
    }

    fn matrix(&mut self) -> Result<Mat3, Box<dyn Error>> {
        Ok(Mat3::from_cols(self.vector()?, self.vector()?, self.vector()?))
    }

    fn moment(&mut self) -> Result<ContactMoment, Box<dyn Error>> {
        let mass = self.number()?;
        let inertia = self.vector()?;
        let rotation = self.matrix()?;
        let scale = self.vector()?;
        let cog = self.vector()?;
        let position = self.vector()?;
        Ok(ContactMoment {
            mass,
            inverse_inertia: inverse_world_inertia(rotation, inertia * scale),
            centre: position + transform_vector(rotation, cog),
            linear_velocity: self.vector()?,
            angular_velocity: self.vector()?,
            elasticity: self.number()?,
            immobile: self.number()? != 0.0,
            breaking_impulse: self.number()?,
            fixed_centre: self.number()? != 0.0,
        })
    }

    fn bounds(&mut self) -> Result<(Obb, Vec3), Box<dyn Error>> {
        let bounds = Obb { centre: self.vector()?, axes: self.matrix()?, half: self.vector()? };
        Ok((bounds, self.vector()?))
    }
}

fn vector(output: &mut dyn Write, value: Vec3) -> io::Result<()> {
    write!(output, " {} {} {}", value.x, value.y, value.z)
}

fn moment(input: &mut Input<'_>, output: &mut dyn Write) -> Result<(), Box<dyn Error>> {
    let iterations: u32 = input.token()?.parse()?;
    let plane = PlaneContact {
        point: input.vector()?,
        normal: input.vector()?,
        friction_static: input.number()?,
        friction_kinetic: input.number()?,
    };
    let (mut a, mut b) = (input.moment()?, input.moment()?);
    let reaction = react_moments(&mut a, &mut b, &plane, iterations);
    let impulse = reaction.map_or(0.0, |r| r.impulse);
    let friction = reaction.map_or(0, |r| match r.friction {
        FrictionState::None => 0,
        FrictionState::Static => 1,
        FrictionState::Dynamic => 2,
    });
    write!(
        output,
        "{} {} {} {} {}",
        u8::from(reaction.is_some()),
        impulse,
        reaction.map_or(0.0, |r| r.closing_speed),
        reaction.map_or(0.0, |r| r.sliding_speed),
        friction
    )?;
    for (own, other) in [(&a, &b), (&b, &a)] {
        vector(output, own.linear_velocity)?;
        vector(output, own.angular_velocity)?;
        write!(output, " {} {}", u8::from(own.immobile), own.notification_delta_v(other, impulse))?;
    }
    Ok(())
}

fn bounds(input: &mut Input<'_>, output: &mut dyn Write) -> Result<(), Box<dyn Error>> {
    let ((a, da), (b, db)) = (input.bounds()?, input.bounds()?);
    let Some(hit) = box_contact(&a, &b, da, db) else {
        write!(output, "0")?;
        return Ok(());
    };
    write!(output, "1")?;
    vector(output, hit.point)?;
    vector(output, hit.normal)?;
    write!(output, " {} {}", hit.overlap, u8::from(hit.point_on_first))?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let mut output = io::BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let line = line?;
        let mut input = Input(line.split_whitespace());
        let kind = input.token()?.to_owned();
        let name = input.token()?.to_owned();
        write!(output, "{name} ")?;
        match kind.as_str() {
            "moment" => moment(&mut input, &mut output)?,
            "box" => bounds(&mut input, &mut output)?,
            _ => return Err(format!("unknown case kind {kind}").into()),
        }
        if input.0.next().is_some() {
            return Err("unexpected extra fields".into());
        }
        writeln!(output)?;
    }
    output.flush()?;
    Ok(())
}
