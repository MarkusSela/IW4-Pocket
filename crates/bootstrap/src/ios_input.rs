//! iOS input bridge: Apple's GameController framework -> bevy gamepad messages, and
//! single-finger touch -> left mouse button (so the menus are tappable).
//! gilrs has no iOS backend, so without this no controller is ever seen.

use bevy::input::ButtonState;
use bevy::input::InputSystems;
use bevy::input::gamepad::{
    GamepadAxis, GamepadButton, GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
    RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use objc2::rc::Retained;
use objc2_game_controller::GCController;

pub struct IosInputPlugin;

impl Plugin for IosInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (poll_controllers, touch_as_mouse).before(InputSystems),
        )
        .add_systems(Update, sync_edit_field_ime);
    }
}

fn sync_edit_field_ime(
    menus: Option<Res<hud::ScriptMenus>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let active = menus.is_some_and(|menus| menus.edit_field_active());
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    if window.ime_enabled != active {
        window.ime_enabled = active;
    }
}

const BUTTONS: [GamepadButton; 17] = [
    GamepadButton::South,
    GamepadButton::East,
    GamepadButton::West,
    GamepadButton::North,
    GamepadButton::LeftTrigger,
    GamepadButton::RightTrigger,
    GamepadButton::LeftTrigger2,
    GamepadButton::RightTrigger2,
    GamepadButton::Select,
    GamepadButton::Start,
    GamepadButton::LeftThumb,
    GamepadButton::RightThumb,
    GamepadButton::DPadUp,
    GamepadButton::DPadDown,
    GamepadButton::DPadLeft,
    GamepadButton::DPadRight,
    GamepadButton::Mode,
];
const AXES: [GamepadAxis; 4] = [
    GamepadAxis::LeftStickX,
    GamepadAxis::LeftStickY,
    GamepadAxis::RightStickX,
    GamepadAxis::RightStickY,
];

struct Pad {
    /// Address of the GCController object: identity only, never dereferenced.
    id: usize,
    entity: Entity,
    buttons: [f32; 17],
    axes: [f32; 4],
    announced: bool,
}

fn read_pad(controller: &GCController) -> Option<([f32; 17], [f32; 4])> {
    // SAFETY: GameController getters are plain property reads on a live controller.
    unsafe {
        let pad = controller.extendedGamepad()?;
        let dpad = pad.dpad();
        let left = pad.leftThumbstick();
        let right = pad.rightThumbstick();
        let optional = |button: Option<Retained<objc2_game_controller::GCControllerButtonInput>>| {
            button.map_or(0.0, |button| button.value())
        };
        let buttons = [
            pad.buttonA().value(),
            pad.buttonB().value(),
            pad.buttonX().value(),
            pad.buttonY().value(),
            pad.leftShoulder().value(),
            pad.rightShoulder().value(),
            pad.leftTrigger().value(),
            pad.rightTrigger().value(),
            optional(pad.buttonOptions()),
            pad.buttonMenu().value(),
            optional(pad.leftThumbstickButton()),
            optional(pad.rightThumbstickButton()),
            dpad.up().value(),
            dpad.down().value(),
            dpad.left().value(),
            dpad.right().value(),
            optional(pad.buttonHome()),
        ];
        let axes = [
            left.xAxis().value(),
            left.yAxis().value(),
            right.xAxis().value(),
            right.yAxis().value(),
        ];
        Some((buttons, axes))
    }
}

fn poll_controllers(
    mut commands: Commands,
    mut pads: Local<Vec<Pad>>,
    mut raw: MessageWriter<RawGamepadEvent>,
    mut connections: MessageWriter<GamepadConnectionEvent>,
    mut button_events: MessageWriter<RawGamepadButtonChangedEvent>,
    mut axis_events: MessageWriter<RawGamepadAxisChangedEvent>,
) {
    // SAFETY: class method returning the current controller list.
    let list = unsafe { GCController::controllers() };
    let mut present: Vec<(usize, Retained<GCController>)> = Vec::new();
    for index in 0..list.count() {
        // SAFETY: index is below count.
        let controller = unsafe { list.objectAtIndex(index) };
        present.push((Retained::as_ptr(&controller) as usize, controller));
    }

    // Disconnected controllers.
    let mut gone = Vec::new();
    for (index, pad) in pads.iter().enumerate() {
        if !present.iter().any(|(id, _)| *id == pad.id) {
            gone.push(index);
        }
    }
    for index in gone.into_iter().rev() {
        let pad = pads.remove(index);
        let event = GamepadConnectionEvent::new(pad.entity, GamepadConnection::Disconnected);
        raw.write(event.clone().into());
        connections.write(event);
        diag::boot_crumb("controller disconnected");
    }

    // New controllers.
    for (id, controller) in &present {
        if pads.iter().any(|pad| pad.id == *id) {
            continue;
        }
        if read_pad(controller).is_none() {
            continue;
        }
        let entity = commands.spawn_empty().id();
        let event = GamepadConnectionEvent::new(
            entity,
            GamepadConnection::Connected {
                name: "iOS controller".to_owned(),
                vendor_id: None,
                product_id: None,
            },
        );
        raw.write(event.clone().into());
        connections.write(event);
        diag::boot_crumb("controller connected");
        pads.push(Pad {
            id: *id,
            entity,
            buttons: [0.0; 17],
            axes: [0.0; 4],
            announced: false,
        });
    }

    // Values: the first frame after connecting only lets the entity materialise.
    for pad in pads.iter_mut() {
        if !pad.announced {
            pad.announced = true;
            continue;
        }
        let Some((_, controller)) = present.iter().find(|(id, _)| *id == pad.id) else {
            continue;
        };
        let Some((buttons, axes)) = read_pad(controller) else {
            continue;
        };
        for (index, value) in buttons.into_iter().enumerate() {
            if (value - pad.buttons[index]).abs() > f32::EPSILON {
                pad.buttons[index] = value;
                let event = RawGamepadButtonChangedEvent::new(pad.entity, BUTTONS[index], value);
                raw.write(event.into());
                button_events.write(event);
            }
        }
        for (index, value) in axes.into_iter().enumerate() {
            if (value - pad.axes[index]).abs() > 0.001 {
                pad.axes[index] = value;
                let event = RawGamepadAxisChangedEvent::new(pad.entity, AXES[index], value);
                raw.write(event.into());
                axis_events.write(event);
            }
        }
    }
}

fn touch_as_mouse(
    mut touches: MessageReader<TouchInput>,
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut buttons: MessageWriter<MouseButtonInput>,
    mut active: Local<Option<u64>>,
    overlay: Option<Res<crate::touch_controls::TouchControlsActive>>,
) {
    let Ok((entity, mut window)) = windows.single_mut() else {
        return;
    };
    // The on-screen controls own every touch while they are shown.
    if overlay.is_some_and(|o| o.0) {
        if let Some(id) = active.take() {
            let _ = id;
            buttons.write(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Released,
                window: entity,
            });
        }
        for _ in touches.read() {}
        return;
    }
    for touch in touches.read() {
        match touch.phase {
            TouchPhase::Started => {
                if active.is_none() {
                    *active = Some(touch.id);
                    window.set_cursor_position(Some(touch.position));
                    buttons.write(MouseButtonInput {
                        button: MouseButton::Left,
                        state: ButtonState::Pressed,
                        window: entity,
                    });
                }
            }
            TouchPhase::Moved if *active == Some(touch.id) => {
                window.set_cursor_position(Some(touch.position));
            }
            TouchPhase::Ended | TouchPhase::Canceled if *active == Some(touch.id) => {
                window.set_cursor_position(Some(touch.position));
                buttons.write(MouseButtonInput {
                    button: MouseButton::Left,
                    state: ButtonState::Released,
                    window: entity,
                });
                *active = None;
            }
            _ => {}
        }
    }
}
