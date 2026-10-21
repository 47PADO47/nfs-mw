//! Export original prop contact parameters for a private reference harness.
//! Output belongs with the user's research, not in the public repository.

use std::error::Error;
use std::io::{self, Write};

use blackbox_attrib::Database;
use game_install::GameDir;
use nfsmw_data::world::PropDynamics;

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or("provide the original installation directory")?;
    let dir = GameDir::open(path)?;
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN")?)?;
    let mut output = io::BufWriter::new(io::stdout().lock());
    if std::env::args().any(|arg| arg == "--vehicles") {
        return vehicles(&db, &mut output);
    }
    let mut count = 0;
    for collection in db.collections_of("smackable") {
        let Some(profile) = PropDynamics::read(collection) else { continue };
        let material = profile.body.objects;
        write!(output, "{} {}", collection.key(), profile.mass)?;
        for values in [
            profile.tensor_scale.to_array(),
            profile.body.cg.to_array(),
            material.elasticity.to_array(),
            material.inertia_scale.to_array(),
        ] {
            for value in values {
                write!(output, " {value}")?;
            }
        }
        writeln!(
            output,
            " {} {} {} {} {} {}",
            material.mass_scale,
            material.friction[0],
            material.friction[1],
            profile.detach_impulse,
            u8::from(profile.immobile),
            u8::from(profile.no_car_effect)
        )?;
        count += 1;
    }
    if count == 0 {
        return Err("installation supplied no readable prop profiles".into());
    }
    output.flush()?;
    Ok(())
}

fn vehicles(db: &Database, output: &mut dyn Write) -> Result<(), Box<dyn Error>> {
    let mut count = 0;
    for vehicle in db.collections_of("pvehicle") {
        let Some(body) = vehicle.follow("rigidbodyspecs") else { continue };
        writeln!(
            output,
            "{} {} {} {}",
            vehicle.key(),
            body.key(),
            vehicle.get_f32("MASS").unwrap_or(0.0),
            u8::from(body.get_bool("IMMOBILE_OBJECT_COLLISIONS").unwrap_or(false))
        )?;
        count += 1;
    }
    if count == 0 {
        return Err("installation supplied no linked vehicle body profiles".into());
    }
    output.flush()?;
    Ok(())
}
