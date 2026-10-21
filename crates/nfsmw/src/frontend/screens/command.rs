//! The screen change a package asks for with a package command.

use blackbox_feng::PackageCommandKind;

use super::{Args, Command};

/// The command a package's own screen change maps to, or `None` for the marker commands the game does not act on.
pub(super) fn package_command(kind: PackageCommandKind, name: String) -> Option<Command> {
    match kind {
        PackageCommandKind::Switch => Some(Command::Switch(name, Args::default())),
        PackageCommandKind::PushGlobal | PackageCommandKind::PushCurrent | PackageCommandKind::PushNone => {
            Some(Command::Push(name, Args::default()))
        }
        PackageCommandKind::Pop => Some(Command::Pop),
        PackageCommandKind::RecordMarker | PackageCommandKind::SwitchToMarker | PackageCommandKind::ClearMarkers => {
            None
        }
    }
}
