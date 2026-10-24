//! The original garage's contextual actions, through the shared input presenter.

use crate::frontend::{logic::Footer, prompts::Hint};
use crate::input::{Action, Bindings, InputDevice};
use crate::ui::prompts::for_action;

pub(in crate::frontend) fn hints(footer: Footer, bindings: &Bindings, device: InputDevice) -> Option<Vec<Hint>> {
    let hint = |action: Action, label| (vec![for_action(bindings, action, device)], label, Some(action));
    let back = hint(Action::MenuBack, "Back");
    let showcase = hint(Action::MenuButton3, "Showcase");
    let mut view_cart = hint(Action::MenuStart, "View Cart");
    if device == InputDevice::Keyboard {
        view_cart.0 = vec![crate::ui::prompts::Prompt::Key("4".into())];
    }
    let result = match footer {
        Footer::GarageManager { add } => {
            let label = match add {
                true => "Add New Car",
                false => "Customize Car",
            };
            let mut hints = vec![back, hint(Action::MenuAccept, label)];
            if !add {
                hints.push(hint(Action::MenuButton1, "Delete Car"));
            }
            hints.push(showcase);
            hints
        }
        Footer::GarageMain { shop } => {
            let mut hints = vec![hint(Action::MenuBack, "Done"), hint(Action::MenuAccept, "Select")];
            if shop {
                hints.push(hint(Action::MenuButton2, "Backroom"));
            }
            hints.push(showcase);
            hints
        }
        Footer::GarageCategories { max_performance } => {
            let mut hints = vec![hint(Action::MenuBack, "Done"), hint(Action::MenuAccept, "Select")];
            if max_performance {
                hints.push(hint(Action::MenuButton2, "Max Performance"));
            }
            hints.push(showcase);
            hints
        }
        Footer::GarageParts => vec![back, hint(Action::MenuAccept, "Add to Cart"), view_cart, showcase],
        Footer::GarageSelect => vec![back, hint(Action::MenuAccept, "Select"), view_cart, showcase],
        Footer::GarageNumbers => vec![
            back,
            hint(Action::MenuAccept, "Add to Cart"),
            hint(Action::MenuButton2, "Remove Numbers"),
            view_cart,
            showcase,
        ],
        Footer::GarageCart { active } => {
            let label = match active {
                true => "Remove",
                false => "Add",
            };
            vec![
                back,
                hint(Action::MenuAccept, "Checkout"),
                hint(Action::MenuButton2, label),
                hint(Action::MenuButton1, "Clear All"),
            ]
        }
        Footer::GarageTuning => vec![
            hint(Action::MenuBack, "Cancel"),
            hint(Action::MenuAccept, "Accept"),
            hint(Action::MenuButton1, "Help"),
        ],
        _ => return None,
    };
    Some(result)
}

/// The package's PC/console captions are replaced by the shared binding-aware footer.
pub(in crate::frontend) fn hide_legacy(tree: &mut blackbox_feng::UiTree) {
    let roots = [
        0xc0a3_2823,
        0x18a4_384f,
        0x8805_e665,
        0xd646_3100,
        0x73dc_b662,
        0x5131_9794,
        0x7344_42b2,
        0xd0d4_95ff,
        0xe084_34fc,
        0x027f_7503,
        0x6385_7de0,
        0xd72f_002a,
        0x9ee0_b6c3,
        0x3f90_8c31,
        0x4170_d94f,
        0x4b89_0cdb,
        0x9de0_9be9,
        0xd57c_95e1,
    ];
    let mut hidden = vec![false; tree.nodes.len()];
    for (index, node) in tree.nodes.iter().enumerate() {
        hidden[index] = roots.contains(&node.name_hash);
        if node.name_hash == 0xe795_717d
            && tree.nodes.iter().any(|sibling| {
                sibling.parent == node.parent
                    && sibling.name_hash == 0xd57c_95e1
                    && sibling.text.as_deref() == Some("Clear All")
            })
        {
            hidden[index] = true;
        }
    }
    for index in 0..tree.nodes.len() {
        let mut parent = tree.nodes[index].parent;
        for _ in 0..tree.nodes.len() {
            let Some(at) = parent.filter(|at| *at < tree.nodes.len()) else { break };
            let footer = [0x84bc_f2b5, 0x17b1_f254, 0xb3a6_9e89].contains(&tree.nodes[at].name_hash);
            if footer
                && (tree.nodes[index].text.is_some()
                    || [0xe795_717d, 0x1ac9_4ca9, 0xfea9_0a33].contains(&tree.nodes[index].name_hash))
            {
                hidden[index] = true;
                break;
            }
            if hidden[at] {
                hidden[index] = true;
                break;
            }
            parent = tree.nodes[at].parent;
        }
    }
    for (node, hide) in tree.nodes.iter_mut().zip(&hidden) {
        node.visible &= !hide;
    }
    tree.draw_order.retain(|index| !hidden[*index]);
}
