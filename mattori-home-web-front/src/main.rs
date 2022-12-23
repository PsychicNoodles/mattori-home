use dioxus::prelude::*;

mod pages;
mod types;

fn main() {
    dioxus::web::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            header {
                class: "container",
                div {
                    class: "headings",
                    h1 {
                        "mattori-home"
                    }
                },
                nav {
                    ul {
                        Link { to: "/ac/control", li { "AC Control" } }
                        Link { to: "/ac/graph", li { "Temperature Graph" } }
                    }
                }
            },
            main {
                class: "container",
                Route {
                    to: "/ac",
                    Route {
                        to: "/control", pages::ac_control {}
                    },
                    Route {
                        to: "/graph", pages::ac_graph {}
                    }
                }
            }
        }
    })
}
