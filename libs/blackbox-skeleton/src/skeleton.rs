//! The bone hierarchy.

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::pose::Pose;
use crate::transform::Transform;

/// One bone: a name, a parent that comes earlier in the list, and the transform it has at rest, relative to the parent.
#[derive(Clone, Debug, PartialEq)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub rest: Transform,
}

/// A list of bones in which every parent precedes its children.
#[derive(Clone, Debug, PartialEq)]
pub struct Skeleton {
    bones: Vec<Bone>,
    index: HashMap<String, usize>,
}

impl Skeleton {
    /// Checks the order of the parents and that names are unique.
    pub fn new(bones: Vec<Bone>) -> Result<Self> {
        let mut index = HashMap::with_capacity(bones.len());
        for (i, bone) in bones.iter().enumerate() {
            if let Some(parent) = bone.parent
                && parent >= i
            {
                return Err(Error::BadParent { bone: i, parent });
            }
            if index.insert(bone.name.clone(), i).is_some() {
                return Err(Error::DuplicateName(bone.name.clone()));
            }
        }
        Ok(Self { bones, index })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bones.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }

    #[must_use]
    pub fn bones(&self) -> &[Bone] {
        &self.bones
    }

    /// The index of the bone with this name.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }

    /// The pose with every bone at rest.
    #[must_use]
    pub fn rest_pose(&self) -> Pose {
        Pose { local: self.bones.iter().map(|b| b.rest).collect() }
    }

    /// Turns local transforms into transforms relative to the skeleton's origin.
    pub fn component_transforms(&self, local: &[Transform], out: &mut Vec<Transform>) -> Result<()> {
        if local.len() != self.bones.len() {
            return Err(Error::CountMismatch { expected: self.bones.len(), got: local.len() });
        }
        out.clear();
        for (i, bone) in self.bones.iter().enumerate() {
            let world = match bone.parent {
                Some(parent) => out[parent].compose(local[i]),
                None => local[i],
            };
            out.push(world);
        }
        Ok(())
    }

    /// `true` when `ancestor` is `bone` or one of its parents.
    #[must_use]
    pub fn is_descendant(&self, bone: usize, ancestor: usize) -> bool {
        let mut at = Some(bone);
        while let Some(i) = at {
            if i == ancestor {
                return true;
            }
            at = self.bones[i].parent;
        }
        false
    }

    /// One weight per bone: `weight` for `root` and everything below it, 0 elsewhere. A layer mask.
    #[must_use]
    pub fn branch_mask(&self, root: usize, weight: f32) -> Vec<f32> {
        (0..self.bones.len()).map(|i| if self.is_descendant(i, root) { weight } else { 0.0 }).collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use glam::{Quat, Vec3};

    /// A chain of four bones, each one unit above its parent.
    pub fn arm() -> Skeleton {
        let up = |y: f32| Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, y, 0.0));
        Skeleton::new(vec![
            Bone { name: "root".into(), parent: None, rest: Transform::IDENTITY },
            Bone { name: "shoulder".into(), parent: Some(0), rest: up(1.0) },
            Bone { name: "elbow".into(), parent: Some(1), rest: up(1.0) },
            Bone { name: "hand".into(), parent: Some(2), rest: up(1.0) },
        ])
        .unwrap()
    }

    #[test]
    fn rejects_forward_parent_and_duplicates() {
        let b = |n: &str, p: Option<usize>| Bone { name: n.into(), parent: p, rest: Transform::IDENTITY };
        assert_eq!(Skeleton::new(vec![b("a", Some(1)), b("b", None)]), Err(Error::BadParent { bone: 0, parent: 1 }));
        assert_eq!(Skeleton::new(vec![b("a", None), b("a", None)]), Err(Error::DuplicateName("a".into())));
    }

    #[test]
    fn component_chain_accumulates() {
        let skel = arm();
        let mut comp = Vec::new();
        skel.component_transforms(&skel.rest_pose().local, &mut comp).unwrap();
        assert!(comp[3].translation.distance(Vec3::new(0.0, 3.0, 0.0)) < 1e-6);
        assert_eq!(skel.find("elbow"), Some(2));
        assert!(skel.component_transforms(&[], &mut comp).is_err());
    }

    #[test]
    fn branch_mask_covers_the_subtree() {
        let skel = arm();
        assert_eq!(skel.branch_mask(2, 1.0), vec![0.0, 0.0, 1.0, 1.0]);
    }
}
