pub mod events;
pub mod key;

use key::Key;

pub enum InputEvent {
    Input(Key),
    Tick,
}
