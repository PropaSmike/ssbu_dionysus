pub const HAVER: u64 = 0x05_1a07_c0e7;
pub const HAVEL: u64 = 0x05_e008_fd84;
pub const HAVE: u64 = 0x04_d27e_ea40;
pub const COMMONHAVE: u64 = 0x0a_d795_40c0;
pub const HASH_MASK: u64 = 0xff_ffff_ffff;
pub const HOLD_WORDS: usize = 8;
pub const BASES_WITHOUT_HAND_BONES: &[&str] = &["pfushigisou"];

const MAIN_WORDS: [usize; 3] = [0, 1, 3];
const SIDE_WORDS: [usize; 2] = [6, 7];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemHand {
    Left,
    Right,
}

impl ItemHand {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    pub const fn bone(self) -> u64 {
        match self {
            Self::Left => HAVEL,
            Self::Right => HAVER,
        }
    }
}

pub fn base_has_hand_bones(base: &str) -> bool {
    !BASES_WITHOUT_HAND_BONES.contains(&base)
}

const fn is_hand(bone: u64) -> bool {
    let bone = bone & HASH_MASK;
    bone == HAVER || bone == HAVEL
}

const fn with_bone(word: u64, bone: u64) -> u64 {
    (word & !HASH_MASK) | bone
}

pub fn remap_bone(word: u64, base_main: u64, hand: ItemHand) -> u64 {
    let main = base_main & HASH_MASK;
    let bone = word & HASH_MASK;
    let side = hand.bone();
    if is_hand(main) {
        if main == side {
            return word;
        }
        return match bone {
            HAVER => with_bone(word, HAVEL),
            HAVEL => with_bone(word, HAVER),
            _ => word,
        };
    }
    if bone == main || bone == HAVE || is_hand(bone) {
        with_bone(word, side)
    } else {
        word
    }
}

pub fn hold_words(base: [u64; HOLD_WORDS], hand: ItemHand) -> Option<[u64; HOLD_WORDS]> {
    let main = base[0];
    let mut words = base;
    for index in MAIN_WORDS {
        words[index] = remap_bone(base[index], main, hand);
    }
    if is_hand(main) {
        for index in SIDE_WORDS {
            words[index] = remap_bone(base[index], main, hand);
        }
    }
    (words != base).then_some(words)
}

pub fn battering_bone(base_bone: u64, base_main: u64, hand: ItemHand) -> Option<u64> {
    let bone = remap_bone(base_bone, base_main, hand);
    (bone != base_bone).then_some(bone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash40;

    const THROW: u64 = 0x05_4f93_4137;
    const LIST: u64 = 0x4fe_1890;

    #[test]
    fn hashes_match_their_names() {
        assert_eq!(hash40("haver"), HAVER);
        assert_eq!(hash40("havel"), HAVEL);
        assert_eq!(hash40("have"), HAVE);
        assert_eq!(hash40("commonhave"), COMMONHAVE);
        assert_eq!(hash40("throw"), THROW);
    }

    #[test]
    fn parses_left_and_right_only() {
        assert_eq!(ItemHand::parse("left"), Some(ItemHand::Left));
        assert_eq!(ItemHand::parse("right"), Some(ItemHand::Right));
        assert_eq!(ItemHand::parse("Left"), None);
        assert_eq!(ItemHand::parse("havel"), None);
        assert_eq!(ItemHand::Left.name(), "left");
    }

    #[test]
    fn right_handed_base_mirrors_to_left() {
        let luigi = [HAVER, HAVER, THROW, HAVER, 9, LIST, HAVEL, HAVEL];
        assert_eq!(
            hold_words(luigi, ItemHand::Left),
            Some([HAVEL, HAVEL, THROW, HAVEL, 9, LIST, HAVER, HAVER])
        );
        assert_eq!(hold_words(luigi, ItemHand::Right), None);
        assert_eq!(battering_bone(HAVER, HAVER, ItemHand::Left), Some(HAVEL));
        assert_eq!(battering_bone(HAVER, HAVER, ItemHand::Right), None);
    }

    #[test]
    fn mirroring_keeps_the_other_hand_word() {
        let link = [HAVEL, HAVEL, THROW, HAVER, 9, LIST, 0, 0];
        assert_eq!(
            hold_words(link, ItemHand::Right),
            Some([HAVER, HAVER, THROW, HAVEL, 9, LIST, 0, 0])
        );
        assert_eq!(hold_words(link, ItemHand::Left), None);
    }

    #[test]
    fn mouth_holder_takes_the_chosen_hand() {
        let pikachu = [HAVE, HAVE, THROW, HAVE, 9, LIST, 0, 0];
        assert_eq!(
            hold_words(pikachu, ItemHand::Right),
            Some([HAVER, HAVER, THROW, HAVER, 9, LIST, 0, 0])
        );
        let mewtwo = [HAVE, HAVE, THROW, HAVE, 9, LIST, HAVE, HAVE];
        assert_eq!(
            hold_words(mewtwo, ItemHand::Left),
            Some([HAVEL, HAVEL, THROW, HAVEL, 9, LIST, HAVE, HAVE])
        );
        assert_eq!(battering_bone(HAVER, HAVE, ItemHand::Left), Some(HAVEL));
        assert_eq!(battering_bone(HAVER, HAVE, ItemHand::Right), None);
        assert_eq!(battering_bone(HAVE, HAVE, ItemHand::Right), Some(HAVER));
    }

    #[test]
    fn upper_bits_survive() {
        let word = (7u64 << 40) | HAVER;
        assert_eq!(remap_bone(word, HAVER, ItemHand::Left), (7u64 << 40) | HAVEL);
        assert_eq!(remap_bone(THROW, HAVER, ItemHand::Left), THROW);
    }

    #[test]
    fn ivysaur_has_no_hand_bones() {
        assert!(!base_has_hand_bones("pfushigisou"));
        assert!(base_has_hand_bones("luigi"));
    }
}
