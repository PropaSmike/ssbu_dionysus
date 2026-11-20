pub const AI_OWNERS: [&str; 94] = [
    "bayonetta", "brave", "buddy", "captain", "chrom", "cloud", "daisy", "dedede", "demon", "diddy", "dolly",
    "donkey", "duckhunt", "edge", "eflame", "elight", "falco", "fox", "gamewatch", "ganon", "gaogaen",
    "gekkouga", "ike", "inkling", "jack", "kamui", "ken", "kirby", "koopa", "koopag", "koopajr", "krool",
    "link", "littlemac", "lucario", "lucas", "lucina", "luigi", "mario", "mariod", "marth", "master",
    "metaknight", "mewtwo", "miienemyf", "miienemyg", "miienemys", "miifighter", "miigunner",
    "miiswordsman", "murabito", "nana", "ness", "packun", "pacman", "palutena", "peach", "pfushigisou",
    "pichu", "pickel", "pikachu", "pikmin", "pit", "pitb", "plizardon", "popo", "purin", "pzenigame",
    "reflet", "richter", "ridley", "robot", "rockman", "rosetta", "roy", "ryu", "samus", "samusd", "sheik",
    "shizue", "shulk", "simon", "snake", "sonic", "szerosuit", "tantan", "toonlink", "trail", "wario",
    "wiifit", "wolf", "yoshi", "younglink", "zelda",
];

pub const OWN_LOGIC: [&str; 7] = ["bayonetta", "donkey", "ken", "pickel", "rosetta", "ryu", "szerosuit"];

pub fn is_ai_owner(name: &str) -> bool {
    AI_OWNERS.contains(&name)
}

pub fn has_own_logic(name: &str) -> bool {
    OWN_LOGIC.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_owner_is_listed_once_and_sorted() {
        let mut sorted = AI_OWNERS.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, AI_OWNERS.to_vec());
        assert!(OWN_LOGIC.iter().all(|name| is_ai_owner(name)));
    }

    #[test]
    fn fighters_without_an_ai_are_not_owners() {
        assert!(is_ai_owner("ryu"));
        assert!(is_ai_owner("pickel"));
        assert!(!is_ai_owner("ptrainer"));
        assert!(!is_ai_owner("common"));
        assert!(!is_ai_owner("Ryu"));
        assert!(has_own_logic("pickel"));
        assert!(!has_own_logic("mario"));
    }
}
