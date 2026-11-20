use pikutracker_core::app::{Action, KeyContext};
use pikutracker_core::keymap::{Key, KeyPress, Keyboard, Keymap, Modifiers, Preset};
use std::collections::BTreeMap;
use std::time::Duration;

const NONE: Modifiers = Modifiers::NONE;

fn keyboard(keys: &[(&str, &str)]) -> Keyboard {
    let keys: BTreeMap<String, String> = keys
        .iter()
        .map(|(action, key)| ((*action).to_owned(), (*key).to_owned()))
        .collect();
    let (keymap, problems) = Keymap::rebound(Preset::PcTracker, &keys);
    assert!(problems.is_empty(), "{problems:?}");
    Keyboard::new(keymap)
}

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

fn down(keyboard: &mut Keyboard, key: Key, at: Duration) -> Option<Action> {
    keyboard.key_down(KeyPress::new(key, NONE), KeyContext::Navigation, false, at)
}

#[test]
fn a_hold_chord_fires_when_its_second_key_goes_down_while_the_first_is_held() {
    let mut keyboard = keyboard(&[("Cut", "Z+X")]);

    assert_eq!(down(&mut keyboard, Key::Char('z'), ms(0)), None);
    assert_eq!(
        down(&mut keyboard, Key::Char('x'), ms(500)),
        Some(Action::Cut)
    );
}

#[test]
fn a_hold_chord_needs_its_first_key_still_held() {
    let mut keyboard = keyboard(&[("Cut", "Z+X")]);

    down(&mut keyboard, Key::Char('z'), ms(0));
    keyboard.key_up(Key::Char('z'));

    assert_eq!(down(&mut keyboard, Key::Char('x'), ms(10)), None);
}

#[test]
fn a_chord_with_an_arrow_still_repeats_while_held() {
    let mut keyboard = keyboard(&[("NudgeUp", "Z+Up")]);
    down(&mut keyboard, Key::Char('z'), ms(0));
    down(&mut keyboard, Key::Up, ms(10));

    let repeated = keyboard.key_down(
        KeyPress::repeated(Key::Up, NONE),
        KeyContext::Navigation,
        false,
        ms(400),
    );

    assert_eq!(
        repeated,
        Some(Action::Nudge {
            up: true,
            coarse: false
        })
    );
}

#[test]
fn a_double_tap_fires_on_the_second_quick_press_of_the_same_key() {
    let mut keyboard = keyboard(&[("NewUnused", "Double-tap Z")]);

    assert_eq!(down(&mut keyboard, Key::Char('z'), ms(0)), None);
    keyboard.key_up(Key::Char('z'));
    assert_eq!(
        down(&mut keyboard, Key::Char('z'), ms(150)),
        Some(Action::NewUnused)
    );
}

#[test]
fn a_slow_second_press_is_not_a_double_tap() {
    let mut keyboard = keyboard(&[("NewUnused", "Double-tap Z")]);

    down(&mut keyboard, Key::Char('z'), ms(0));
    keyboard.key_up(Key::Char('z'));

    assert_eq!(down(&mut keyboard, Key::Char('z'), ms(1000)), None);
}

#[test]
fn another_key_between_the_taps_breaks_the_double_tap() {
    let mut keyboard = keyboard(&[("NewUnused", "Double-tap Z")]);

    down(&mut keyboard, Key::Char('z'), ms(0));
    keyboard.key_up(Key::Char('z'));
    down(&mut keyboard, Key::Down, ms(50));
    keyboard.key_up(Key::Down);

    assert_eq!(down(&mut keyboard, Key::Char('z'), ms(100)), None);
}

#[test]
fn a_third_quick_tap_starts_a_new_double_tap_rather_than_firing_again() {
    let mut keyboard = keyboard(&[("NewUnused", "Double-tap Z")]);

    let taps: Vec<_> = [0, 100, 200, 300]
        .into_iter()
        .map(|at| {
            let action = down(&mut keyboard, Key::Char('z'), ms(at));
            keyboard.key_up(Key::Char('z'));
            action
        })
        .collect();

    assert_eq!(
        taps,
        [None, Some(Action::NewUnused), None, Some(Action::NewUnused)]
    );
}

#[test]
fn single_presses_still_work_beside_chords_and_double_taps() {
    let mut keyboard = keyboard(&[("Cut", "Z+X"), ("NewUnused", "Double-tap Z")]);

    assert_eq!(
        keyboard.key_down(
            KeyPress::new(Key::Char('s'), Modifiers::CTRL),
            KeyContext::Navigation,
            false,
            ms(0)
        ),
        Some(Action::Save)
    );
}

#[test]
fn a_binding_while_selecting_applies_only_while_a_selection_is_active() {
    let mut keyboard = Keyboard::new(Keymap::m8_style());
    let x = KeyPress::new(Key::Char('x'), NONE);

    keyboard.key_down(x, KeyContext::Navigation, true, ms(0));
    assert_eq!(keyboard.key_up(Key::Char('x')), Some(Action::Copy));
    keyboard.key_down(x, KeyContext::Navigation, false, ms(1000));
    assert_eq!(keyboard.key_up(Key::Char('x')), None);
}

#[test]
fn a_key_that_starts_chords_acts_when_it_goes_up_alone() {
    let mut keyboard = Keyboard::new(Keymap::m8_style());
    let z = KeyPress::new(Key::Char('z'), NONE);

    assert_eq!(
        keyboard.key_down(z, KeyContext::Navigation, true, ms(0)),
        None
    );
    assert_eq!(keyboard.key_up(Key::Char('z')), Some(Action::Clone));
}

#[test]
fn holding_z_for_a_chord_while_selecting_cuts_without_cloning() {
    let mut keyboard = Keyboard::new(Keymap::m8_style());

    let z = keyboard.key_down(
        KeyPress::new(Key::Char('z'), NONE),
        KeyContext::Navigation,
        true,
        ms(0),
    );
    let x = keyboard.key_down(
        KeyPress::new(Key::Char('x'), NONE),
        KeyContext::Navigation,
        true,
        ms(10),
    );
    let released = [
        keyboard.key_up(Key::Char('x')),
        keyboard.key_up(Key::Char('z')),
    ];

    assert_eq!((z, x, released), (None, Some(Action::Cut), [None, None]));
}

#[test]
fn a_chord_beats_a_typed_key_of_the_cursors_context() {
    let mut keyboard = Keyboard::new(Keymap::m8_style());
    keyboard.key_down(
        KeyPress::new(Key::Char('x'), NONE),
        KeyContext::PhrasePitch,
        false,
        ms(0),
    );

    let t = keyboard.key_down(
        KeyPress::new(Key::Char('t'), NONE),
        KeyContext::PhrasePitch,
        false,
        ms(10),
    );

    assert_eq!(t, Some(Action::ToggleCart));
}

#[test]
fn a_file_name_takes_every_letter_even_a_chords_held_key() {
    let mut keyboard = Keyboard::new(Keymap::m8_style());

    let typed: Vec<_> = ['z', 'x']
        .into_iter()
        .map(|ch| {
            keyboard.key_down(
                KeyPress::new(Key::Char(ch), NONE),
                KeyContext::FileName,
                false,
                ms(0),
            )
        })
        .collect();

    assert_eq!(
        typed,
        [Some(Action::TypeChar('z')), Some(Action::TypeChar('x'))]
    );
}
