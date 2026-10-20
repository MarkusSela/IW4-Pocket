//! Prontezza arma per il caricamento selettivo (flat data, sim non dipende da assets).
//! `active == false` => comportamento identico al precaricamento completo.
use std::collections::BTreeSet;
use std::sync::Mutex;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeaponReadiness {
    pub ready: Vec<bool>,
    pub active: bool,
}

impl WeaponReadiness {
    pub fn new(ready: Vec<bool>, active: bool) -> Self {
        Self { ready, active }
    }

    /// True solo se il modo selettivo e attivo e l'arma e nota come non pronta.
    /// Id fuori tabella: considerato pronto (non cambia il comportamento precedente).
    pub fn is_pending(&self, id: u32) -> bool {
        self.active && self.ready.get(id as usize) == Some(&false)
    }
}

static LOGGED: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

/// Logga "preparazione" una sola volta per arma. Ritorna true se ha loggato.
pub(crate) fn log_preparing_once(id: u32, context: &str) -> bool {
    let first = LOGGED.lock().map(|mut s| s.insert(id)).unwrap_or(false);
    if first {
        diag::info!(Sim, "weapon {} in preparazione ({}): differito", id, context);
    }
    first
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_never_pending() {
        let r = WeaponReadiness::new(vec![true, false], false);
        assert!(!r.is_pending(1));
    }

    #[test]
    fn active_pending_only_for_false_rows() {
        let r = WeaponReadiness::new(vec![true, false], true);
        assert!(!r.is_pending(0));
        assert!(r.is_pending(1));
        assert!(!r.is_pending(9));
    }

    #[test]
    fn default_is_inactive() {
        assert!(!WeaponReadiness::default().is_pending(0));
    }
}
