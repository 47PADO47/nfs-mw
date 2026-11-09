//! Running the `vehicle`, `gear` and `devtools` console commands.

use super::dev_parse::{DevCommand, DevtoolsCommand, GearCommand, VehicleCommand};
use crate::app::Host;
use crate::devtools::panels::{DevPanels, Panel, Request};
use crate::devtools::vehicle;
use crate::garage::options::{Change, VehicleOptions};

const NONE: &str = "no vehicle on show (drive starts one in the world; view-car shows one)";

pub(super) fn run(command: DevCommand, host: &mut Host, dev: &mut DevPanels) -> Result<String, String> {
    match command {
        DevCommand::Vehicle(command) => run_vehicle(command, host, dev),
        DevCommand::Gear(command) => run_gear(command, host),
        DevCommand::Devtools(command) => run_devtools(command, host, dev),
    }
}

fn run_vehicle(command: VehicleCommand, host: &mut Host, dev: &mut DevPanels) -> Result<String, String> {
    let change = match command {
        VehicleCommand::Panel => {
            dev.request(Request::Open { panel: Panel::Vehicle, from_menu: false });
            return Ok("the vehicle panel is open (F11 closes it)".into());
        }
        VehicleCommand::Slots => return Ok(options(host)?.slots_text()),
        VehicleCommand::Groups => return Ok(options(host)?.groups_text()),
        VehicleCommand::Set { slot, choice } => Change::Set { slot, choice },
        VehicleCommand::Reset => Change::Reset,
        VehicleCommand::Group { name, on: Some(on) } => Change::Group { name, on },
        VehicleCommand::Group { name, on: None } => Change::ResetGroup(name),
        VehicleCommand::Color(color) => Change::Color(color),
    };
    let renderer = host.renderer.as_mut().ok_or("the renderer is not ready")?;
    let scene = host.scene.vehicle().ok_or(NONE)?;
    scene.change(renderer.as_mut(), &change)
}

fn options(host: &mut Host) -> Result<VehicleOptions, String> {
    host.scene.vehicle().and_then(|scene| scene.options()).ok_or_else(|| NONE.to_owned())
}

fn run_gear(command: GearCommand, host: &mut Host) -> Result<String, String> {
    vehicle::ensure_gear_catalogue(host.scene.vehicle().and_then(|scene| scene.install_dir()));
    match command {
        GearCommand::List => vehicle::gear_list(),
        GearCommand::Set { part, choice } => vehicle::set_gear(part, &choice),
        GearCommand::Stock => vehicle::stock_gear(),
    }
}

fn run_devtools(command: DevtoolsCommand, host: &mut Host, dev: &mut DevPanels) -> Result<String, String> {
    match command {
        DevtoolsCommand::Close => {
            dev.request(Request::CloseAll);
            Ok(String::new())
        }
        DevtoolsCommand::Press(button) => {
            dev.pressed.push(button);
            dev.simulated = true;
            Ok(String::new())
        }
        DevtoolsCommand::Open(Panel::Garage) if host.scene.garage().is_none() => {
            Err("this scene has no garage".to_owned())
        }
        DevtoolsCommand::Open(panel) => {
            dev.request(Request::Open { panel, from_menu: false });
            Ok(format!("{} ({})", panel.label(), panel.key()))
        }
    }
}
