use bevy::{
    ecs::{
        event::{Event, EventReader, EventWriter},
        system::{Commands, Res, Resource},
    },
    input::{
        gamepad::{
            Gamepad, GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType, GamepadEvent,
        },
        Axis, Input,
    },
    math::Vec2,
    reflect::Struct,
    utils::HashSet,
};

#[derive(Resource)]
pub struct MyGamepad(Gamepad);

pub fn gamepad_connections(
    mut commands: Commands,
    my_gamepad: Option<Res<MyGamepad>>,
    mut gamepad_evr: EventReader<GamepadEvent>,
) {
    for ev in gamepad_evr.read() {
        match &ev {
            GamepadEvent::Connection(info) if info.connected() => {
                println!(
                    "New gamepad connected with ID: {:?}, name: {}",
                    info.gamepad.id,
                    info.gamepad.name_at(0).unwrap_or_default()
                );

                // if we don't have any gamepad yet, use this one
                if my_gamepad.is_none() {
                    commands.insert_resource(MyGamepad(info.gamepad));
                }
            }

            GamepadEvent::Connection(info) if info.disconnected() => {
                println!("Lost gamepad connection with ID: {:?}", info.gamepad.id,);

                // if it's the one we previously associated with the player,
                // disassociate it:
                if let Some(MyGamepad(old_gamepad)) = my_gamepad.as_deref() {
                    if *old_gamepad == info.gamepad {
                        commands.remove_resource::<MyGamepad>();
                    }
                }
            }
            // other events are irrelevant
            _ => {}
        }
    }
}

#[derive(Event, Default)]
pub struct PlayerInputEvent {
    pub xy: Option<Vec2>,
    pub dir: Option<Vec2>,
    pub keys: HashSet<GamepadButtonType>,
}

pub fn gamepad_input(
    axes: Res<Axis<GamepadAxis>>,
    buttons: Res<Input<GamepadButton>>,
    my_gamepad: Option<Res<MyGamepad>>,
    mut player_input: EventWriter<PlayerInputEvent>,
) {
    let mut player_input_event = PlayerInputEvent::default();
    let mut some_input = false;

    let gamepad = if let Some(gp) = my_gamepad {
        gp.0
    } else {
        return;
    };

    let axis_lx = GamepadAxis {
        gamepad,
        axis_type: GamepadAxisType::LeftStickX,
    };
    let axis_ly = GamepadAxis {
        gamepad,
        axis_type: GamepadAxisType::LeftStickY,
    };

    let axis_rx = GamepadAxis {
        gamepad,
        axis_type: GamepadAxisType::RightStickX,
    };
    let axis_ry = GamepadAxis {
        gamepad,
        axis_type: GamepadAxisType::RightStickY,
    };

    if let (Some(x), Some(y)) = (axes.get(axis_lx), axes.get(axis_ly)) {
        let left_stick_pos = Vec2::new(x, y).normalize();

        if left_stick_pos.length() > 0.5 {
            player_input_event.xy = Some(left_stick_pos);
            some_input = true;
        }
    }

    if let (Some(x), Some(y)) = (axes.get(axis_rx), axes.get(axis_ry)) {
        let right_stick_pos = Vec2::new(x, y).normalize();

        if right_stick_pos.length() > 0.8 {
            player_input_event.dir = Some(right_stick_pos);
            some_input = true;
        }
    }

    for button_type in [
        GamepadButtonType::South,
        GamepadButtonType::East,
        GamepadButtonType::North,
        GamepadButtonType::West,
    ] {
        if buttons.pressed(GamepadButton {
            gamepad,
            button_type,
        }) {
            player_input_event.keys.insert(button_type);
            some_input = true;
        }
    }

    for button_type in [
        GamepadButtonType::LeftTrigger,
        GamepadButtonType::RightTrigger,
        GamepadButtonType::LeftTrigger2,
        GamepadButtonType::RightTrigger2,
    ] {
        if buttons.just_pressed(GamepadButton {
            gamepad,
            button_type,
        }) {
            player_input_event.keys.insert(button_type);
            some_input = true;
        }
    }

    if some_input {
        player_input.send(player_input_event);
    }
}
