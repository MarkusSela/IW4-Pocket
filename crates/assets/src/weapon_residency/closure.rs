//! Weapon dependency registry and class closure.

use std::collections::{BTreeSet, HashMap};
use std::fmt;

pub type WeaponId = u32;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeaponDeps {
    pub weapon: String,
    pub gun_xmodels: Vec<String>,
    pub hand_xmodels: Vec<String>,
    pub world_xmodels: Vec<String>,
    pub projectile_xmodels: Vec<String>,
    pub xanims: Vec<String>,
    pub sounds: Vec<String>,
    pub fx: Vec<String>,
    pub materials: Vec<String>,
}

impl WeaponDeps {
    pub fn named(weapon: &str) -> Self {
        Self { weapon: weapon.to_string(), ..Self::default() }
    }

    fn xmodels(&self) -> impl Iterator<Item = &String> {
        self.gun_xmodels
            .iter()
            .chain(&self.hand_xmodels)
            .chain(&self.world_xmodels)
            .chain(&self.projectile_xmodels)
    }
}

/// Weapon name -> deps. Ids are insertion indices and are never renumbered.
#[derive(Clone, Debug, Default)]
pub struct WeaponRegistry {
    entries: Vec<WeaponDeps>,
    by_name: HashMap<String, WeaponId>,
}

impl WeaponRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert; an already-known name keeps its id AND its original deps.
    pub fn insert(&mut self, deps: WeaponDeps) -> WeaponId {
        if let Some(&id) = self.by_name.get(&deps.weapon) {
            return id;
        }
        let id = self.entries.len() as WeaponId;
        self.by_name.insert(deps.weapon.clone(), id);
        self.entries.push(deps);
        id
    }

    pub fn id(&self, name: &str) -> Option<WeaponId> {
        self.by_name.get(name).copied()
    }

    pub fn get(&self, name: &str) -> Option<&WeaponDeps> {
        self.id(name).map(|i| &self.entries[i as usize])
    }

    pub fn by_id(&self, id: WeaponId) -> Option<&WeaponDeps> {
        self.entries.get(id as usize)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// In id order.
    pub fn iter(&self) -> impl Iterator<Item = (WeaponId, &WeaponDeps)> {
        self.entries.iter().enumerate().map(|(i, d)| (i as WeaponId, d))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClassRequest {
    pub primary: String,
    pub secondary: String,
    pub lethal: String,
    pub tactical: String,
    /// Weapon-variant names (e.g. attachment builds); must exist in the registry.
    pub attachments: Vec<String>,
    /// Camo material names; added to the closure materials as-is.
    pub camo: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClosureError {
    MissingWeapon(String),
}

impl fmt::Display for ClosureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClosureError::MissingWeapon(n) => write!(f, "unknown weapon {n:?}"),
        }
    }
}

impl std::error::Error for ClosureError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceKind {
    Weapon,
    XModel,
    XAnim,
    Sound,
    Fx,
    Material,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeaponClosure {
    pub weapons: BTreeSet<String>,
    pub xmodels: BTreeSet<String>,
    pub xanims: BTreeSet<String>,
    pub sounds: BTreeSet<String>,
    pub fx: BTreeSet<String>,
    pub materials: BTreeSet<String>,
    /// Tutti i modelli/anim che il registro conosce come risorse di un'arma (selezionata o no).
    /// Il filtro in cattura salta SOLO cio che e noto qui e non e nella closure: modelli
    /// sconosciuti (attachment, scope, mappa) restano sempre catturati.
    pub known_xmodels: BTreeSet<String>,
    pub known_xanims: BTreeSet<String>,
}

impl WeaponClosure {
    /// All (kind, name) members, deterministic order.
    pub fn resources(&self) -> Vec<(ResourceKind, String)> {
        let mut v = Vec::new();
        let sets: [(ResourceKind, &BTreeSet<String>); 6] = [
            (ResourceKind::Weapon, &self.weapons),
            (ResourceKind::XModel, &self.xmodels),
            (ResourceKind::XAnim, &self.xanims),
            (ResourceKind::Sound, &self.sounds),
            (ResourceKind::Fx, &self.fx),
            (ResourceKind::Material, &self.materials),
        ];
        for (k, s) in sets {
            v.extend(s.iter().map(|n| (k, n.clone())));
        }
        v
    }

    fn add_weapon(&mut self, d: &WeaponDeps) {
        self.weapons.insert(d.weapon.clone());
        self.xmodels.extend(d.xmodels().cloned());
        self.xanims.extend(d.xanims.iter().cloned());
        self.sounds.extend(d.sounds.iter().cloned());
        self.fx.extend(d.fx.iter().cloned());
        self.materials.extend(d.materials.iter().cloned());
    }
}

/// Union of the deps of every weapon named by `classes` plus `extra_always`.
/// An unknown weapon is an explicit error, never skipped. Empty names (unused slots) are skipped.
pub fn closure(
    registry: &WeaponRegistry,
    classes: &[ClassRequest],
    extra_always: &[String],
) -> Result<WeaponClosure, ClosureError> {
    let mut out = WeaponClosure::default();
    for (_, d) in registry.iter() {
        out.known_xmodels.extend(d.xmodels().cloned());
        out.known_xanims.extend(d.xanims.iter().cloned());
    }
    let add = |name: &str, out: &mut WeaponClosure| -> Result<(), ClosureError> {
        if name.is_empty() {
            return Ok(());
        }
        let d = registry
            .get(name)
            .ok_or_else(|| ClosureError::MissingWeapon(name.to_string()))?;
        out.add_weapon(d);
        Ok(())
    };
    for name in extra_always {
        add(name, &mut out)?;
    }
    for c in classes {
        for name in [&c.primary, &c.secondary, &c.lethal, &c.tactical] {
            add(name, &mut out)?;
        }
        for name in &c.attachments {
            add(name, &mut out)?;
        }
        out.materials.extend(c.camo.iter().filter(|m| !m.is_empty()).cloned());
    }
    Ok(out)
}

/// Capture-time filter: is this asset part of the closure?
pub fn filter_allows(c: &WeaponClosure, kind: ResourceKind, name: &str) -> bool {
    let set = match kind {
        ResourceKind::Weapon => &c.weapons,
        ResourceKind::XModel => &c.xmodels,
        ResourceKind::XAnim => &c.xanims,
        ResourceKind::Sound => &c.sounds,
        ResourceKind::Fx => &c.fx,
        ResourceKind::Material => &c.materials,
    };
    set.contains(name)
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;

    pub fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    pub fn deps(name: &str) -> WeaponDeps {
        WeaponDeps {
            weapon: name.into(),
            gun_xmodels: s(&[&format!("{name}_gun")]),
            hand_xmodels: s(&["viewhands"]),
            world_xmodels: s(&[&format!("{name}_world")]),
            projectile_xmodels: vec![],
            xanims: s(&[&format!("{name}_fire")]),
            sounds: s(&[&format!("{name}_snd")]),
            fx: s(&["muzzle"]),
            materials: s(&[&format!("{name}_mat")]),
        }
    }

    pub fn registry(names: &[&str]) -> WeaponRegistry {
        let mut r = WeaponRegistry::new();
        for n in names {
            r.insert(deps(n));
        }
        r
    }

    pub fn class(p: &str, sec: &str) -> ClassRequest {
        ClassRequest { primary: p.into(), secondary: sec.into(), ..Default::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::*;
    use super::*;

    #[test]
    fn closure_unions_all_classes_and_always() {
        let r = registry(&["m4", "usp", "ak", "knife"]);
        let mut c1 = class("m4", "usp");
        c1.camo = s(&["camo_red"]);
        let c2 = class("ak", "");
        let c = closure(&r, &[c1, c2], &s(&["knife"])).unwrap();
        assert_eq!(c.weapons.len(), 4);
        for w in ["m4", "usp", "ak", "knife"] {
            assert!(filter_allows(&c, ResourceKind::XModel, &format!("{w}_gun")));
            assert!(filter_allows(&c, ResourceKind::XAnim, &format!("{w}_fire")));
            assert!(filter_allows(&c, ResourceKind::Sound, &format!("{w}_snd")));
            assert!(filter_allows(&c, ResourceKind::Material, &format!("{w}_mat")));
        }
        assert!(filter_allows(&c, ResourceKind::Fx, "muzzle"));
        assert!(filter_allows(&c, ResourceKind::Material, "camo_red"));
        assert_eq!(c.xmodels.iter().filter(|m| *m == "viewhands").count(), 1);
    }

    #[test]
    fn known_sets_cover_whole_registry() {
        let r = registry(&["m4", "ak"]);
        let c = closure(&r, &[class("m4", "")], &[]).unwrap();
        assert!(c.known_xmodels.contains("ak_gun") && c.known_xanims.contains("ak_fire"));
        assert!(!c.known_xmodels.contains("m4_acog"));
    }

    #[test]
    fn closure_excludes_unrequested() {
        let r = registry(&["m4", "ak"]);
        let c = closure(&r, &[class("m4", "")], &[]).unwrap();
        assert!(!filter_allows(&c, ResourceKind::Weapon, "ak"));
        assert!(!filter_allows(&c, ResourceKind::XModel, "ak_gun"));
    }

    #[test]
    fn unknown_weapon_is_explicit_error() {
        let r = registry(&["m4"]);
        assert_eq!(
            closure(&r, &[class("m4", "nope")], &[]),
            Err(ClosureError::MissingWeapon("nope".into()))
        );
        assert_eq!(
            closure(&r, &[], &s(&["ghost"])),
            Err(ClosureError::MissingWeapon("ghost".into()))
        );
        let mut c = class("m4", "");
        c.attachments = s(&["m4_acog"]);
        assert!(closure(&r, &[c], &[]).is_err());
    }

    #[test]
    fn ids_stable_after_additions() {
        let mut r = registry(&["a", "b", "c"]);
        let before: Vec<_> = ["a", "b", "c"].iter().map(|n| r.id(n).unwrap()).collect();
        assert_eq!(before, vec![0, 1, 2]);
        assert_eq!(r.insert(deps("d")), 3);
        assert_eq!(r.insert(deps("a")), 0, "re-insert keeps id");
        assert_eq!(r.len(), 4);
        let after: Vec<_> = ["a", "b", "c"].iter().map(|n| r.id(n).unwrap()).collect();
        assert_eq!(before, after);
        assert_eq!(r.by_id(3).unwrap().weapon, "d");
    }
}
