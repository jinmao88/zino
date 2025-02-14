use crate::{class::Class, extension::VNodeExt, format_class};
use dioxus::prelude::*;
use dioxus_router::components::{IntoRoutable, Link};

/// A responsive navigation header.
pub fn Navbar<'a>(cx: Scope<'a, NavbarProps<'a>>) -> Element {
    let children = cx.props.children.as_ref()?;
    let class = format_class!(cx, "navbar is-link");
    if children.has_component("NavbarBrand") {
        render! {
            nav {
                class: "{class}",
                children
            }
        }
    } else {
        render! {
            nav {
                class: "{class}",
                NavbarMenu {
                    children
                }
            }
        }
    }
}

/// The [`Navbar`] properties struct for the configuration of the component.
#[derive(Props)]
pub struct NavbarProps<'a> {
    /// The class attribute for the component.
    #[props(into)]
    pub class: Option<Class<'a>>,
    /// The children to render within the component.
    children: Element<'a>,
}

/// A container for the logo and optionally some links or icons.
pub fn NavbarBrand<'a>(cx: Scope<'a, NavbarBrandProps<'a>>) -> Element {
    let class = format_class!(cx, "navbar-brand");
    render! {
        div {
            class: "{class}",
            &cx.props.children
        }
    }
}

/// The [`NavbarBrand`] properties struct for the configuration of the component.
#[derive(Props)]
pub struct NavbarBrandProps<'a> {
    /// The class attribute for the component.
    #[props(into)]
    pub class: Option<Class<'a>>,
    /// The children to render within the component.
    children: Element<'a>,
}

/// A horizontal menu used in the navigation header.
pub fn NavbarMenu<'a>(cx: Scope<'a, NavbarMenuProps<'a>>) -> Element {
    let class = format_class!(cx, "navbar-menu is-active");
    render! {
        div {
            class: "{class}",
            &cx.props.children
        }
    }
}

/// The [`NavbarMenu`] properties struct for the configuration of the component.
#[derive(Props)]
pub struct NavbarMenuProps<'a> {
    /// The class attribute for the component.
    #[props(into)]
    pub class: Option<Class<'a>>,
    /// The children to render within the component.
    children: Element<'a>,
}

/// The left section of the navbar menu.
pub fn NavbarStart<'a>(cx: Scope<'a, NavbarStartProps<'a>>) -> Element {
    let class = format_class!(cx, "navbar-start");
    render! {
        div {
            class: "{class}",
            &cx.props.children
        }
    }
}

/// The [`NavbarStart`] properties struct for the configuration of the component.
#[derive(Props)]
pub struct NavbarStartProps<'a> {
    /// The class attribute for the component.
    #[props(into)]
    pub class: Option<Class<'a>>,
    /// The children to render within the component.
    children: Element<'a>,
}

/// The middle section of the navbar menu.
pub fn NavbarCenter<'a>(cx: Scope<'a, NavbarCenterProps<'a>>) -> Element {
    let class = format_class!(cx, "navbar-center");
    render! {
        div {
            class: "{class}",
            &cx.props.children
        }
    }
}

/// The [`NavbarCenter`] properties struct for the configuration of the component.
#[derive(Props)]
pub struct NavbarCenterProps<'a> {
    /// The class attribute for the component.
    #[props(into)]
    pub class: Option<Class<'a>>,
    /// The children to render within the component.
    children: Element<'a>,
}

/// The right section of the navbar menu.
pub fn NavbarEnd<'a>(cx: Scope<'a, NavbarEndProps<'a>>) -> Element {
    let class = format_class!(cx, "navbar-end");
    render! {
        div {
            class: "{class}",
            &cx.props.children
        }
    }
}

/// The [`NavbarEnd`] properties struct for the configuration of the component.
#[derive(Props)]
pub struct NavbarEndProps<'a> {
    /// The class attribute for the component.
    #[props(into)]
    pub class: Option<Class<'a>>,
    /// The children to render within the component.
    children: Element<'a>,
}

/// A link to navigate to another route in the navigation header.
pub fn NavbarLink<'a>(cx: Scope<'a, NavbarLinkProps<'a>>) -> Element {
    let class = format_class!(cx, "navbar-item");
    let active_class = format_class!(cx, "is-active");
    render! {
        Link {
            class: "{class}",
            active_class: "{active_class}",
            to: cx.props.to.clone(),
            &cx.props.children
        }
    }
}

/// The [`NavbarLink`] properties struct for the configuration of the component.
#[derive(Props)]
pub struct NavbarLinkProps<'a> {
    /// The class attribute for the component.
    #[props(into)]
    pub class: Option<Class<'a>>,
    /// A class to apply to the generate HTML anchor tag if the `target` route is active.
    #[props(into)]
    pub active_class: Option<Class<'a>>,
    /// The navigation target. Roughly equivalent to the href attribute of an HTML anchor tag.
    #[props(into)]
    pub to: IntoRoutable,
    /// The children to render within the component.
    children: Element<'a>,
}
