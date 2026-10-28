//! The original garage's contextual actions, through the shared input presenter.

use crate::frontend::logic::Footer;
use crate::frontend::prompts::{CLEAR_ALL, Hint, HintLabel as L};
use crate::input::{Action, Bindings, InputDevice};
use crate::ui::prompts::for_action;

pub(in crate::frontend) fn hints(footer: Footer, bindings: &Bindings, device: InputDevice) -> Option<Vec<Hint>> {
    let hint = |action: Action, label| (vec![for_action(bindings, action, device)], label, Some(action));
    let back = hint(Action::MenuBack, L::Back);
    let showcase = hint(Action::MenuButton3, L::Showcase);
    let mut view_cart = hint(Action::MenuStart, L::ViewCart);
    if device == InputDevice::Keyboard {
        view_cart.0 = vec![crate::ui::prompts::Prompt::Key("4".into())];
    }
    let result = match footer {
        Footer::GarageManager { add, showcase: can_showcase } => {
            let label = match add {
                true => L::AddNewCar,
                false => L::CustomizeCar,
            };
            let mut hints = vec![back, hint(Action::MenuAccept, label)];
            if !add {
                hints.push(hint(Action::MenuButton1, L::DeleteCar));
            }
            if can_showcase {
                hints.push(showcase);
            }
            hints
        }
        Footer::GarageMain { shop } => {
            let mut hints = vec![hint(Action::MenuBack, L::Done), hint(Action::MenuAccept, L::Select)];
            if shop {
                hints.push(hint(Action::MenuButton2, L::Backroom));
            }
            hints.push(showcase);
            hints
        }
        Footer::GarageCategories { max_performance } => {
            let mut hints = vec![hint(Action::MenuBack, L::Done), hint(Action::MenuAccept, L::Select)];
            if max_performance {
                hints.push(hint(Action::MenuButton2, L::MaxPerformance));
            }
            hints.push(showcase);
            hints
        }
        Footer::GarageParts => vec![back, hint(Action::MenuAccept, L::AddToCart), view_cart, showcase],
        Footer::GarageSelect => vec![back, hint(Action::MenuAccept, L::Select), view_cart, showcase],
        Footer::GarageNumbers => vec![
            back,
            hint(Action::MenuAccept, L::AddToCart),
            hint(Action::MenuButton2, L::RemoveNumbers),
            view_cart,
            showcase,
        ],
        Footer::GarageCart { active } => {
            let label = match active {
                true => L::Remove,
                false => L::Add,
            };
            vec![
                back,
                hint(Action::MenuAccept, L::Checkout),
                hint(Action::MenuButton2, label),
                hint(Action::MenuButton1, L::ClearAll),
            ]
        }
        Footer::GarageTuning => vec![
            hint(Action::MenuBack, L::Cancel),
            hint(Action::MenuAccept, L::Accept),
            hint(Action::MenuButton1, L::Help),
        ],
        Footer::GarageTuningHelp => vec![hint(Action::MenuBack, L::Hide)],
        _ => return None,
    };
    Some(result)
}

/// The package's PC/console captions are replaced by the shared binding-aware footer.
pub(in crate::frontend) fn hide_legacy(tree: &mut blackbox_feng::UiTree, footer: Footer) {
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
    // The package's Clear All caption, in the current language: the draw tree keeps no labels.
    let clear_all = crate::i18n::game(CLEAR_ALL);
    let mut hidden = vec![false; tree.nodes.len()];
    for (index, node) in tree.nodes.iter().enumerate() {
        // Help's close animation re-enables these complete, separately rooted PC hints.
        let tuning_pc = matches!(footer, Footer::GarageTuning | Footer::GarageTuningHelp)
            && [0xd9e6_3061, 0x9502_d148, 0x5232_50e9, 0x64bc_dff2].contains(&node.name_hash);
        hidden[index] = roots.contains(&node.name_hash) || tuning_pc;
        if node.name_hash == 0xe795_717d
            && tree.nodes.iter().any(|sibling| {
                sibling.parent == node.parent
                    && sibling.name_hash == 0xd57c_95e1
                    && sibling.text.is_some()
                    && sibling.text == clear_all
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
