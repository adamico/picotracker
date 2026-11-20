use crate::cart::SLOTS;

const PAIR: usize = 2;
const SHORTEST_SFX_RANGE: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClaimPart {
    Sfx,
    Patterns,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ClaimText {
    part: ClaimPart,
    text: String,
}

fn digits_since_last_separator(text: &str) -> &str {
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

fn first_and_last_slot(token: &str) -> Option<(u8, u8)> {
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

    pub(super) fn type_digit(&mut self, digit: u8) {
        if digits_since_last_separator(&self.text).len() == PAIR {
            match self.part {
                ClaimPart::Sfx => self.text.push(' '),
                ClaimPart::Patterns if self.text.contains('-') => return,
                ClaimPart::Patterns => self.text.push('-'),
            }
        }
        self.text.push_str(&format!("{digit:X}"));
    }

    pub(super) fn type_dash(&mut self) {
        let token = self.text.rsplit(' ').next().unwrap_or("");
        if self.part == ClaimPart::Sfx && token.len() == PAIR && !token.contains('-') {
            self.text.push('-');
        }
    }

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

    pub(super) fn sfx_slots(&self) -> Option<Vec<u8>> {
        let mut slots = Vec::new();
        for token in self.text.split(' ').filter(|token| !token.is_empty()) {
            let (first, last) = first_and_last_slot(token)?;
            slots.extend(first..=last);
        }
        slots.sort_unstable();
        slots.dedup();
        Some(slots)
    }

    pub(super) fn pattern_range(&self) -> Option<Option<(u8, u8)>> {
        if self.text.is_empty() {
            return Some(None);
        }
        first_and_last_slot(&self.text).map(Some)
    }

    pub(super) fn is_whole_claim(&self) -> bool {
        match self.part {
            ClaimPart::Sfx => self.sfx_slots().is_some(),
            ClaimPart::Patterns => self.pattern_range().is_some(),
        }
    }
}
