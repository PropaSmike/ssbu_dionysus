use crate::item_kinds::item_kind;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemRule {
    pub start_all: bool,
    pub off: Vec<i32>,
    pub on: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolved {
    pub rule: ItemRule,
    pub unknown: Vec<String>,
}

pub fn resolve(start_all: bool, off: &[String], on: &[String]) -> Resolved {
    let mut out = Resolved {
        rule: ItemRule {
            start_all,
            ..Default::default()
        },
        unknown: Vec::new(),
    };
    for (names, kinds) in [(off, &mut out.rule.off), (on, &mut out.rule.on)] {
        for name in names {
            match item_kind(name) {
                Some(kind) if !kinds.contains(&kind) => kinds.push(kind),
                Some(_) => {}
                None => out.unknown.push(name.clone()),
            }
        }
    }
    out
}

pub fn row(rule: &ItemRule, base: &[u8], mapping: &[i32]) -> Vec<u8> {
    let length = base.len().min(mapping.len());
    (0..length)
        .map(|index| {
            let kind = mapping[index];
            if rule.off.contains(&kind) {
                0
            } else if rule.on.contains(&kind) {
                1
            } else if rule.start_all {
                1
            } else {
                base[index]
            }
        })
        .collect()
}

pub fn switched_off(row: &[u8]) -> usize {
    row.iter().filter(|flag| **flag == 0).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn names_resolve_to_item_kinds_and_unknown_ones_are_kept_aside() {
        let resolved = resolve(false, &names(&["mushroom", "item_kind_moon", "Lunala", "nope"]), &names(&["warpstar"]));
        assert_eq!(resolved.rule.off, vec![22, 228, 339]);
        assert_eq!(resolved.rule.on, vec![26]);
        assert_eq!(resolved.unknown, names(&["nope"]));
    }

    #[test]
    fn a_name_listed_twice_counts_once() {
        let resolved = resolve(false, &names(&["moon", "moon"]), &[]);
        assert_eq!(resolved.rule.off, vec![228]);
    }

    #[test]
    fn the_donor_row_is_kept_where_the_author_says_nothing() {
        let rule = ItemRule { start_all: false, off: vec![22], on: vec![26] };
        let base = [1, 1, 0, 0, 1];
        let mapping = [21, 22, 25, 26, 228];
        assert_eq!(row(&rule, &base, &mapping), vec![1, 0, 0, 1, 1]);
    }

    #[test]
    fn starting_from_all_ignores_the_donor_row() {
        let rule = ItemRule { start_all: true, off: vec![228], on: vec![] };
        assert_eq!(row(&rule, &[0, 0, 0], &[22, 25, 228]), vec![1, 1, 0]);
    }

    #[test]
    fn off_wins_over_on() {
        let rule = ItemRule { start_all: true, off: vec![22], on: vec![22] };
        assert_eq!(row(&rule, &[1], &[22]), vec![0]);
    }

    #[test]
    fn the_row_never_outgrows_the_mapping_or_the_base() {
        let rule = ItemRule::default();
        assert_eq!(row(&rule, &[1, 0, 1], &[22, 25]).len(), 2);
        assert_eq!(row(&rule, &[1], &[22, 25]).len(), 1);
        assert_eq!(switched_off(&[1, 0, 0, 1]), 2);
    }
}
