use std::collections::BTreeMap;

use super::super::rules::{ParityExclusionUnit, RulesParityExclusions};

#[derive(Debug)]
pub(in super::super) struct ParityExclusionsIndex {
    pub(in super::super) commands: BTreeMap<Vec<String>, ParityExclusionUnit>,
    pub(in super::super) flags: BTreeMap<(Vec<String>, String), ParityExclusionUnit>,
    pub(in super::super) args: BTreeMap<(Vec<String>, String), ParityExclusionUnit>,
}

pub(in super::super) fn build_parity_exclusions_index(
    exclusions: &RulesParityExclusions,
) -> ParityExclusionsIndex {
    let mut commands = BTreeMap::new();
    let mut flags = BTreeMap::new();
    let mut args = BTreeMap::new();

    for unit in &exclusions.units {
        match unit.unit.as_str() {
            "command" => {
                commands.insert(unit.path.clone(), unit.clone());
            }
            "flag" => {
                if let Some(key) = unit.key.as_ref() {
                    flags.insert((unit.path.clone(), key.clone()), unit.clone());
                }
            }
            "arg" => {
                if let Some(name) = unit.name.as_ref() {
                    args.insert((unit.path.clone(), name.clone()), unit.clone());
                }
            }
            _ => {}
        }
    }

    ParityExclusionsIndex {
        commands,
        flags,
        args,
    }
}
