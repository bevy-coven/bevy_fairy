//! The machine plate: the family API standard, checked against `FairyPlugin`.
//!
//! Headless: `MinimalPlugins`, `InputPlugin` (which the plugin requires) and the plugin.
//!
//! Names come from `System::name()`, which is the full type path only with Bevy's `debug`
//! feature; the dev-dependency on `bevy` turns it on, and the test asserts it saw at least one
//! `bevy_fairy::` system so a build without it fails instead of passing vacuously.
//!
//! The crate has no settings resources (none of its public resources is named `*Settings`), so
//! the settings checks are the assertion that none is reflected as a resource by accident.

use bevy::app::App;
use bevy::ecs::reflect::{AppTypeRegistry, ReflectResource};
use bevy::ecs::schedule::{Schedules, SystemSet};
use bevy::prelude::*;
use bevy_fairy::{FairyPlugin, FairySystems};

const CRATE: &str = "bevy_fairy::";

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::input::InputPlugin,
        FairyPlugin::default(),
    ));
    app.update();
    app
}

/// Every system of the crate must be in one of the crate's root-set variants.
#[test]
fn every_system_is_in_the_root_set() {
    let app = app();
    let roots = [FairySystems.intern()];
    let schedules = app.world().resource::<Schedules>();
    let mut seen = 0;
    let mut outside = Vec::new();
    for (label, schedule) in schedules.iter() {
        let Ok(systems) = schedule.systems() else {
            continue;
        };
        for (key, system) in systems {
            let name = system.name().to_string();
            if !name.starts_with(CRATE) {
                continue;
            }
            seen += 1;
            let in_root = roots.iter().any(|root| {
                schedule
                    .graph()
                    .systems_in_set(*root)
                    .is_ok_and(|set| set.contains(&key))
            });
            if !in_root {
                outside.push(format!("{name} in {label:?}"));
            }
        }
    }
    assert!(
        seen > 0,
        "no `{CRATE}` systems seen: is Bevy's `debug` feature on?"
    );
    assert!(
        outside.is_empty(),
        "systems outside FairySystems: {outside:#?}"
    );
}

/// Every public component is reflected and registered.
#[cfg(feature = "plugin-guide")]
#[test]
fn components_are_reflected() {
    use bevy::ecs::reflect::ReflectComponent;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::input::InputPlugin,
        FairyPlugin::default(),
    ));
    app.update();
    let registry = app.world().resource::<AppTypeRegistry>().read();
    let registration = registry
        .get(std::any::TypeId::of::<bevy_fairy::GuideOverlay>())
        .expect("GuideOverlay is registered by FairyPlugin");
    assert!(
        registration.data::<ReflectComponent>().is_some(),
        "GuideOverlay has no ReflectComponent"
    );
}

/// The crate has no settings resources: none of its resources is reflected as one.
#[test]
fn no_settings_resources() {
    let app = app();
    let registry = app.world().resource::<AppTypeRegistry>().read();
    let resources = registry
        .iter()
        .filter(|r| r.data::<ReflectResource>().is_some())
        .filter(|r| r.type_info().type_path().starts_with(CRATE))
        .map(|r| r.type_info().type_path().to_owned())
        .collect::<Vec<_>>();
    assert!(
        resources.is_empty(),
        "unexpected reflected resources: {resources:?}"
    );
}
