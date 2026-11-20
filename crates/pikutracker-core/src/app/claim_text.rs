//! The Claim as the Cart view's fields hold it: SFX as hex slots and ranges such as `03 04 32-3D`,
//! and Patterns as one range such as `18-1D`. Separators are added as the digits are typed, so
//! only `-` is ever typed, for SFX ranges.

use crate::cart::SLOTS;

const PAIR: usize = 2;
const SHORTEST_SFX_RANGE: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClaimPart {
    Sfx,
    Patterns,
}

/// The text of one Claim field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ClaimText {
    part: ClaimPart,
    text: String,
}

/// The digits typed since the last separator.
fn last_number(text: &str) -> &str {
    text.rsplit([' ', '-']).next().unwrap_or("")
}

fn slot(digits: &str) -> Option<u8> {
    if digits.len() != PAIR {
        return None;
    }
    u8::from_str_radix(digits, 16)
        .ok()
        .filter(|slot| usize::from(*slot) < SLOTS)
}

/// A slot, `XX`, or a range of slots, `XX-YY`, as its first and last slot.
fn range(token: &str) -> Option<(u8, u8)> {
    match token.split_once('-') {
        Some((first, last)) => {
            let (first, last) = (slot(first)?, slot(last)?);
            (first <= last).then_some((first, last))
        }
        None => slot(token).map(|only| (only, only)),
    }
}

fn range_text((first, last): (u8, u8)) -> String {
    if first == last {
        format!("{first:02X}")
    } else {
        format!("{first:02X}-{last:02X}")
    }
}

/// Slots as text, such as `03 04 32-3D`: runs of three or more as ranges, and the rest one by one.
pub(super) fn slots_text(sfx: &[u8]) -> String {
    let mut runs: Vec<(u8, u8)> = Vec::new();
    for &slot in sfx {
        match runs.last_mut() {
            Some((_, last)) if *last + 1 == slot => *last = slot,
            _ => runs.push((slot, slot)),
        }
    }
    let mut tokens = Vec::new();
    for (first, last) in runs {
        if usize::from(last - first) + 1 >= SHORTEST_SFX_RANGE {
            tokens.push(range_text((first, last)));
        } else {
            tokens.extend((first..=last).map(|slot| range_text((slot, slot))));
        }
    }
    tokens.join(" ")
}

/// Slots named with the word for one or for several of them, such as `PATTERNS 18-1D`, unless
/// there are none.
pub(super) fn named_slots(slots: &[u8], one: &str, many: &str) -> Option<String> {
    let what = if slots.len() == 1 { one } else { many };
    (!slots.is_empty()).then(|| format!("{what} {}", slots_text(slots)))
}

pub(super) fn patterns_text(patterns: Option<(u8, u8)>) -> String {
    patterns.map_or_else(String::new, range_text)
}

impl ClaimText {
    pub(super) fn sfx(sfx: &[u8]) -> Self {
        Self {
            part: ClaimPart::Sfx,
            text: slots_text(sfx),
        }
    }

    pub(super) fn patterns(patterns: Option<(u8, u8)>) -> Self {
        Self {
            part: ClaimPart::Patterns,
            text: patterns_text(patterns),
        }
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    /// Types one hex digit. A finished pair of digits gets a separator first: a space between
    /// SFX, and the dash of the range between Patterns.
    pub(super) fn type_digit(&mut self, digit: u8) {
        if last_number(&self.text).len() == PAIR {
            match self.part {
                ClaimPart::Sfx => self.text.push(' '),
                ClaimPart::Patterns if self.text.contains('-') => return,
                ClaimPart::Patterns => self.text.push('-'),
            }
        }
        self.text.push_str(&format!("{digit:X}"));
    }

    /// Types the dash of an SFX range after its first slot.
    pub(super) fn type_dash(&mut self) {
        let token = self.text.rsplit(' ').next().unwrap_or("");
        if self.part == ClaimPart::Sfx && token.len() == PAIR && !token.contains('-') {
            self.text.push('-');
        }
    }

    /// Takes back the last character typed, with any separator added before it.
    pub(super) fn backspace(&mut self) {
        self.text.pop();
        let added = match self.part {
            ClaimPart::Sfx => self.text.ends_with(' '),
            ClaimPart::Patterns => self.text.ends_with('-'),
        };
        if added {
            self.text.pop();
        }
    }

    /// The SFX slots the text names, in ascending order, if it is a whole Claim.
    pub(super) fn sfx_slots(&self) -> Option<Vec<u8>> {
        let mut slots = Vec::new();
        for token in self.text.split(' ').filter(|token| !token.is_empty()) {
            let (first, last) = range(token)?;
            slots.extend(first..=last);
        }
        slots.sort_unstable();
        slots.dedup();
        Some(slots)
    }

    /// The range of Patterns the text names, if it is a whole Claim. Empty text claims none.
    pub(super) fn pattern_range(&self) -> Option<Option<(u8, u8)>> {
        if self.text.is_empty() {
            return Some(None);
        }
        range(&self.text).map(Some)
    }

    /// Whether the text is a whole Claim.
    pub(super) fn is_valid(&self) -> bool {
        match self.part {
            ClaimPart::Sfx => self.sfx_slots().is_some(),
            ClaimPart::Patterns => self.pattern_range().is_some(),
        }
    }
}
