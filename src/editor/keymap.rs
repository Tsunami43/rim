use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Operator;
use super::action::Action;

/// A key as a keymap lookup key: code + modifiers.
/// A dedicated type (instead of `KeyEvent`) so we can ignore the noisy
/// fields (`kind`/`state`) and normalize the modifiers.
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub struct KeyBind {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

impl KeyBind {
    /// Turn an event into a lookup key.
    /// SHIFT is dropped: it is already encoded in the character itself
    /// (`G` vs `g`), and terminals report it inconsistently — otherwise
    /// capital-letter binds would sometimes fail to match.
    pub fn from_event(key: KeyEvent) -> Self {
        let mods = key.modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT);
        Self {
            code: key.code,
            mods,
        }
    }
}

/// The keymap as data: a "key -> action" table.
pub struct Keymap {
    normal: HashMap<KeyBind, Action>,
}

impl Keymap {
    pub fn default_vim() -> Self {
        let none = KeyModifiers::NONE;
        let ctrl = KeyModifiers::CONTROL;

        let mut normal = HashMap::new();
        let mut bind = |code: KeyCode, mods: KeyModifiers, action: Action| {
            normal.insert(KeyBind { code, mods }, action);
        };

        // motions
        bind(KeyCode::Char('h'), none, Action::MoveLeft);
        bind(KeyCode::Char('l'), none, Action::MoveRight);
        bind(KeyCode::Char('k'), none, Action::MoveUp);
        bind(KeyCode::Char('j'), none, Action::MoveDown);
        bind(KeyCode::Char('w'), none, Action::WordForward(false));
        bind(KeyCode::Char('W'), none, Action::WordForward(true));
        bind(KeyCode::Char('b'), none, Action::WordBackward(false));
        bind(KeyCode::Char('B'), none, Action::WordBackward(true));
        bind(KeyCode::Char('e'), none, Action::WordEnd(false));
        bind(KeyCode::Char('E'), none, Action::WordEnd(true));
        bind(KeyCode::Char('0'), none, Action::LineStart);
        bind(KeyCode::Char('$'), none, Action::LineEnd);
        bind(KeyCode::Char('^'), none, Action::FirstNonBlank);
        bind(KeyCode::Char('G'), none, Action::GotoBottom);
        bind(KeyCode::Char('u'), ctrl, Action::HalfPageUp);
        bind(KeyCode::Char('d'), ctrl, Action::HalfPageDown);

        // modes
        bind(KeyCode::Char('i'), none, Action::InsertBefore);
        bind(KeyCode::Char('a'), none, Action::InsertAfter);
        bind(KeyCode::Char('I'), none, Action::InsertLineStart);
        bind(KeyCode::Char('A'), none, Action::InsertLineEnd);
        bind(KeyCode::Char('o'), none, Action::OpenLineBelow);
        bind(KeyCode::Char('O'), none, Action::OpenLineAbove);
        bind(KeyCode::Char(':'), none, Action::EnterCommand);

        // editing
        bind(KeyCode::Char('x'), none, Action::DeleteChar);
        bind(KeyCode::Char('D'), none, Action::DeleteToLineEnd);
        bind(KeyCode::Char('J'), none, Action::JoinLines);
        bind(KeyCode::Char('~'), none, Action::ToggleCase);
        bind(KeyCode::Char('r'), none, Action::ReplaceChar);
        bind(KeyCode::Char('d'), none, Action::StartOperator(Operator::Delete));

        // system
        bind(KeyCode::Char('s'), ctrl, Action::Save);
        bind(KeyCode::Char('q'), ctrl, Action::Quit);

        Self { normal }
    }

    /// Look up the action bound to a key in Normal mode.
    pub fn lookup_normal(&self, bind: &KeyBind) -> Option<Action> {
        self.normal.get(bind).copied()
    }
}
