use dioxus::prelude::*;

use crate::types::AcState;

#[derive(Props, PartialEq)]
pub struct AcControlProps {
    ac_state: Option<AcState>,
}

pub fn ac_control(cx: Scope) -> Element {
    cx.render(rsx! {
        article {
            h2 {
                "AC Control"
            },
            h3 {
                "Current state"
            }
        }
    })
}
