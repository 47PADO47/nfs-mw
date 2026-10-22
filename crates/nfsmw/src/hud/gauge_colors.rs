//! Apply optional RGB overrides after authored scripts, preserving their alpha.
use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

pub(super) struct Binding {
    objects: [Vec<(ObjectRef, [u8; 3])>; 3],
    active: [bool; 3],
}

impl Binding {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        let tree = rt.tree(package);
        let mut groups = [
            vec![rt.find(package, fe_hash_upper("TAC_Lines_7500")), rt.find_guid(package, 0x456e)],
            ["3rdPersonNeedle", "3rdperson_TurboDial"].map(|n| rt.find(package, fe_hash_upper(n))).to_vec(),
            ["SPEED_DIGIT_1", "SPEED_DIGIT_2", "SPEED_DIGIT_3", "3rdPersonSpeedUnits", "3rdPersonGear"]
                .map(|n| rt.find(package, fe_hash_upper(n)))
                .to_vec(),
        ];
        groups[0].extend(tree.nodes.iter().filter_map(|n| {
            matches!(n.kind, blackbox_feng::NodeKind::Image { texture, .. } if texture == fe_hash_upper("TURBO_LINES"))
                .then_some(Some(ObjectRef { package, index: n.index }))
        }));
        let objects = groups.map(|group| {
            group
                .into_iter()
                .flatten()
                .filter_map(|o| {
                    let n = tree.nodes.iter().find(|n| n.index == o.index)?;
                    let rgb = match n.name_hash == fe_hash_upper("3rdPersonGear") {
                        true => [0; 3], // The live tachometer binding uses black for this digit.
                        false => [n.colour[0], n.colour[1], n.colour[2]],
                    };
                    Some((o, rgb))
                })
                .collect()
        });
        Self { objects, active: [false; 3] }
    }

    pub fn apply(&mut self, rt: &mut Runtime, colors: [Option<[u8; 3]>; 3]) {
        for (i, color) in colors.into_iter().enumerate() {
            if color.is_none() && !self.active[i] {
                continue;
            }
            for (object, original) in &self.objects[i] {
                rt.set_colour(*object, color.unwrap_or(*original));
            }
            self.active[i] = color.is_some();
        }
    }
}
