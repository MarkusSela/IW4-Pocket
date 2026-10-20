//! Filtro in cattura del caricamento selettivo delle armi.
//!
//! Il walker legge sempre il corpo dell'asset (lo stream avanza comunque): il filtro
//! decide solo se il sink CATTURA (skel/mesh/anim) o salta. Solo modelli/anim di arma
//! non presenti nella closure vengono saltati; mappa, player body, fx, ecc. restano sempre.
//! Un modello saltato che riappare come Offset/alias non trova nome in `xmodel_names`:
//! viene contato come "alias non residente", senza panic ne modello fantasma.

use crate::weapon_residency::{ResourceKind, WeaponClosure, filter_allows};
use std::collections::BTreeSet;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CaptureCounters {
    pub xmodel_captured: usize,
    pub xmodel_skipped: usize,
    pub xanim_captured: usize,
    pub xanim_skipped: usize,
    pub alias_unresolved: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct CaptureFilter {
    closure: WeaponClosure,
    counters: CaptureCounters,
    /// Nomi saltati (non residenti), per diagnosi e dedup del log.
    skipped: BTreeSet<String>,
}

/// Prefisso delle anim di arma in prima persona (unico indicatore sicuro lato XAnim).
const FPV_ANIM_PREFIX: &str = "viewmodel_";

impl CaptureFilter {
    pub(crate) fn new(closure: WeaponClosure) -> Self {
        Self {
            closure,
            counters: CaptureCounters::default(),
            skipped: BTreeSet::new(),
        }
    }

    /// Un modello e "di arma" se model_kind lo classifica Fpv o WorldWeapon.
    /// Soldier e non classificati (mappa, fx, ecc.) sono sempre catturati.
    pub(crate) fn is_weapon_model(name: &str) -> bool {
        matches!(
            asset_model::model_kind(name),
            Some(asset_model::ModelKind::Fpv | asset_model::ModelKind::WorldWeapon)
        )
    }

    pub(crate) fn allow_xmodel(&mut self, name: &str) -> bool {
        // Salta solo cio che il registro conosce come modello di un'arma NON selezionata.
        // Attachment/scope e modelli sconosciuti restano catturati.
        if !Self::is_weapon_model(name)
            || !self.closure.known_xmodels.contains(name)
            || filter_allows(&self.closure, ResourceKind::XModel, name)
        {
            self.counters.xmodel_captured += 1;
            return true;
        }
        self.counters.xmodel_skipped += 1;
        self.skipped.insert(name.to_owned());
        false
    }

    pub(crate) fn allow_xanim(&mut self, name: &str) -> bool {
        if !name.starts_with(FPV_ANIM_PREFIX)
            || !self.closure.known_xanims.contains(name)
            || filter_allows(&self.closure, ResourceKind::XAnim, name)
        {
            self.counters.xanim_captured += 1;
            return true;
        }
        self.counters.xanim_skipped += 1;
        self.skipped.insert(name.to_owned());
        false
    }

    pub(crate) fn note_alias_unresolved(&mut self) {
        self.counters.alias_unresolved += 1;
    }

    #[cfg(test)]
    pub(crate) fn is_skipped(&self, name: &str) -> bool {
        self.skipped.contains(name)
    }

    #[cfg(test)]
    pub(crate) fn counters(&self) -> CaptureCounters {
        self.counters
    }

    pub(crate) fn summary(&self) -> String {
        let c = self.counters;
        format!(
            "selective capture: xmodel captured={} skipped={}, xanim captured={} skipped={}, alias-of-skipped={}",
            c.xmodel_captured, c.xmodel_skipped, c.xanim_captured, c.xanim_skipped, c.alias_unresolved
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(models: &[&str], anims: &[&str]) -> CaptureFilter {
        let mut c = WeaponClosure::default();
        c.xmodels.extend(models.iter().map(|s| s.to_string()));
        c.xanims.extend(anims.iter().map(|s| s.to_string()));
        // Armi note al registro ma non selezionate.
        c.known_xmodels.extend(
            ["viewmodel_ak47", "viewmodel_m4", "weapon_m4"].map(String::from),
        );
        c.known_xanims.extend(["viewmodel_ak47_fire", "viewmodel_m4_fire"].map(String::from));
        CaptureFilter::new(c)
    }

    #[test]
    fn keeps_non_weapon_models_always() {
        let mut f = filter(&[], &[]);
        assert!(f.allow_xmodel("com_barrel_blue"));
        assert!(f.allow_xmodel("mp_body_ally_ranger_assault"));
        assert_eq!(f.counters().xmodel_skipped, 0);
    }

    #[test]
    fn skips_unselected_weapon_models_only() {
        let mut f = filter(&["viewmodel_ak47"], &[]);
        assert!(f.allow_xmodel("viewmodel_ak47"));
        assert!(!f.allow_xmodel("viewmodel_m4"));
        assert!(!f.allow_xmodel("weapon_m4"));
        assert_eq!(f.counters().xmodel_skipped, 2);
        assert!(f.is_skipped("weapon_m4"));
    }

    #[test]
    fn unknown_weapon_like_models_are_kept() {
        // Attachment/scope: nome da arma ma non nel registro => mai saltato.
        let mut f = filter(&[], &[]);
        assert!(f.allow_xmodel("viewmodel_m4_acog"));
        assert!(f.allow_xmodel("weapon_m4_silencer"));
        assert!(f.allow_xanim("viewmodel_m4_acog_fire"));
        assert_eq!(f.counters().xmodel_skipped, 0);
    }

    #[test]
    fn xanim_only_fpv_anims_filtered() {
        let mut f = filter(&[], &["viewmodel_ak47_fire"]);
        assert!(f.allow_xanim("viewmodel_ak47_fire"));
        assert!(!f.allow_xanim("viewmodel_m4_fire"));
        assert!(f.allow_xanim("pb_stand_alert"));
    }
}
