//! Punti di integrazione del caricamento selettivo (P1 metadati + tabella pronta).
//!
//! P1 (scelta piu semplice): il registro dipendenze si deriva dal WeaponCatalog gia
//! costruito (accessor gun/hand/world/projectile/rocket/sz_xanims) invece di un walk
//! leggero dedicato. Suoni/fx/materiali non vengono ancora derivati (closure parziale:
//! il filtro in cattura agisce solo su XModel/XAnim di arma).

use crate::weapon_residency::{
    ClassRequest, ClosureError, Residency, WeaponClosure, WeaponDeps, WeaponRegistry, closure,
};
use asset_game::WeaponRegistry as WeaponCatalog;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// Nome normalizzato (minuscolo, senza suffisso `_mp`), come fa il catalogo.
fn normalize_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    lower.strip_suffix("_mp").unwrap_or(&lower).to_owned()
}

/// Namespace (`iw4`, `t5`, ...) con almeno un'arma nel registro.
fn installed_namespaces(registry: &WeaponRegistry) -> BTreeSet<String> {
    registry
        .iter()
        .filter_map(|(_, d)| d.weapon.split_once(':').map(|(ns, _)| ns.to_owned()))
        .collect()
}

/// `Some(ns)` se `raw` e qualificato con un namespace NON installato: quell'arma non puo
/// esistere in questo catalogo, quindi si salta (con log). Un'arma ignota in un namespace
/// PRESENTE resta invece un errore visibile.
fn uninstalled_namespace(installed: &BTreeSet<String>, raw: &str) -> Option<String> {
    let (ns, _) = raw.split_once(':')?;
    let ns = ns.to_ascii_lowercase();
    (!installed.contains(&ns)).then_some(ns)
}

/// Chiave di registro `<namespace>:<nome normalizzato>`; senza namespace solo il nome.
/// Evita collisioni tra T5/IW4/IW5 con lo stesso nome base e il problema `_mp`.
fn registry_key(ns: Option<asset_core::AssetNamespace>, name: &str) -> String {
    match ns {
        Some(ns) => format!("{}:{}", ns.as_str(), normalize_name(name)),
        None => normalize_name(name),
    }
}

/// Chiave di registro di una riga del catalogo (None = riga vuota).
fn catalog_key(catalog: &WeaponCatalog, index: u32) -> Option<String> {
    let name = catalog.name_of(index);
    (!name.is_empty()).then(|| registry_key(catalog.identity_namespace_of(index), name))
}

/// Registro in ordine di catalogo, chiavi `<ns>:<nome>` (vedi `registry_key`).
pub fn registry_from_catalog(catalog: &WeaponCatalog) -> WeaponRegistry {
    let mut registry = WeaponRegistry::new();
    for index in 1..=catalog.len() as u32 {
        let Some(key) = catalog_key(catalog, index) else {
            continue;
        };
        let mut deps = WeaponDeps::named(&key);
        let one = |v: Option<&str>| v.map(|s| vec![s.to_owned()]).unwrap_or_default();
        deps.gun_xmodels = one(catalog.gun_xmodel_of(index));
        deps.hand_xmodels = one(catalog.hand_xmodel_of(index));
        deps.world_xmodels = one(catalog.world_model_of(index));
        deps.projectile_xmodels = catalog
            .projectile_model_of(index)
            .into_iter()
            .chain(catalog.rocket_model_of(index))
            .map(str::to_owned)
            .collect();
        if let Some(slots) = catalog.sz_xanims_of(index) {
            deps.xanims = slots.iter().flatten().cloned().collect();
        }
        registry.insert(deps);
    }
    registry
}

/// Tabella piatta indicizzata per ID CATALOGO (indice 0 = nessuna arma = pronta).
/// Gli id non presenti nel registro risultano pronti (nessun cambio di comportamento).
pub fn ready_table_by_catalog(
    catalog: &WeaponCatalog,
    registry: &WeaponRegistry,
    residency: &Residency,
) -> Vec<bool> {
    let by_registry = residency.ready_table(registry);
    let mut out = vec![true; catalog.len() + 1];
    for index in 1..=catalog.len() as u32 {
        if let Some(key) = catalog_key(catalog, index)
            && let Some(id) = registry.id(&key)
            && let Some(&ready) = by_registry.get(id as usize)
        {
            out[index as usize] = ready;
        }
    }
    out
}

/// Selezione attiva: closure + digest stabile (chiave di cache del common set).
#[derive(Debug, Clone)]
pub struct Selection {
    pub digest: u64,
    pub closure: WeaponClosure,
    pub classes: usize,
}

/// Hash stabile (DefaultHasher::new() ha chiavi fisse, BTreeSet e ordinato).
pub fn selection_digest(closure: &WeaponClosure) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (kind, name) in closure.resources() {
        (kind as u8).hash(&mut hasher);
        name.hash(&mut hasher);
    }
    hasher.finish()
}

/// Risolve un nome preset ("t5:weapon/x_mp", "iw4:weapon/y_mp", "z_mp", "z") nelle chiavi di
/// registro. Qualificato (`ns:`): una sola chiave `ns:<nome senza _mp>`. Non qualificato:
/// TUTTE le chiavi con quel nome base (superset sicuro, mai un'arma di un altro namespace
/// scelta a caso). Vuoto se non trovato.
fn resolve_names(registry: &WeaponRegistry, raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }
    if registry.get(raw).is_some() {
        return vec![raw.to_owned()];
    }
    let (ns, rest) = match raw.split_once(':') {
        Some((ns, rest)) => (Some(ns.to_ascii_lowercase()), rest),
        None => (None, raw),
    };
    let base = normalize_name(rest.rsplit('/').next().unwrap_or(rest));
    match ns {
        Some(ns) => {
            let key = format!("{ns}:{base}");
            if registry.get(&key).is_some() { vec![key] } else { Vec::new() }
        }
        None => registry
            .iter()
            .filter(|(_, d)| d.weapon == base || d.weapon.rsplit(':').next() == Some(base.as_str()))
            .map(|(_, d)| d.weapon.clone())
            .collect(),
    }
}

/// Armi di default sempre residenti (coltello/melee, killstreak, bomba SD): solo quelle che
/// il registro conosce davvero. Un nome assente NON fa fallire la selezione (vedi
/// `resolve_names`: vuoto => ignorato). Le killstreak sono tutte quelle `killstreak_*`.
pub fn default_extra_always(registry: &WeaponRegistry) -> Vec<String> {
    // Weapons the simulation hands out by name, outside any class: melee, objective items,
    // killstreak and insertion carriers, and the stand-ins the script host uses for launchers
    // and shields (sim/script/host/players.rs, sim/equipment.rs). A weapon that is not loaded is
    // refused by `give` and ignored by pickups, so these must always be resident.
    const FIXED: [&str; 16] = [
        "knife_mp",
        "defaultweapon_mp",
        "briefcase_bomb_mp",
        "briefcase_bomb_defuse_mp",
        "throwingknife_mp",
        "frag_grenade_mp",
        "fists_mp",
        "airdrop_marker_mp",
        "flare_mp",
        "smoke_grenade_mp",
        "flash_grenade_mp",
        "riotshield_mp",
        "stinger_mp",
        "javelin_mp",
        "usp_mp",
        "m4_mp",
    ];
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut absent: Vec<&str> = Vec::new();
    for raw in FIXED {
        let found = resolve_names(registry, raw);
        if found.is_empty() {
            absent.push(raw);
        }
        out.extend(found);
    }
    if !absent.is_empty() {
        // Not an error (the catalog may not carry every stand-in), but never silent.
        diag::info!(World, "selective weapons: always-resident weapons not in the catalog: {absent:?}");
    }
    for (_, d) in registry.iter() {
        let base = d.weapon.rsplit(':').next().unwrap_or(&d.weapon);
        if base.starts_with("killstreak_") || base.ends_with("_killstreak") {
            out.insert(d.weapon.clone());
        }
    }
    out.into_iter().collect()
}

/// Primo risultato; gli altri in `extra`. Non trovato: nome vuoto (saltato, e registrato in
/// `skipped`) se il namespace non e installato, altrimenti il nome grezzo, che produrra un
/// `MissingWeapon` visibile.
fn resolve_slot(
    registry: &WeaponRegistry,
    installed: &BTreeSet<String>,
    raw: &str,
    extra: &mut Vec<String>,
    skipped: &mut BTreeSet<String>,
) -> String {
    let mut all = resolve_names(registry, raw).into_iter();
    match all.next() {
        Some(first) => {
            extra.extend(all);
            first
        }
        None if uninstalled_namespace(installed, raw).is_some() => {
            skipped.insert(raw.to_owned());
            String::new()
        }
        None => raw.to_owned(),
    }
}

/// Classi -> closure -> digest. Include le varianti con attachment (nome arma + `_`)
/// perche i preset indicano gli attachment come codici, non come nomi di arma.
pub fn build_selection(
    registry: &WeaponRegistry,
    classes: &[ClassRequest],
    extra_always: &[String],
) -> Result<Selection, ClosureError> {
    let installed = installed_namespaces(registry);
    let mut skipped: BTreeSet<String> = BTreeSet::new();
    let classes: Vec<ClassRequest> = classes
        .iter()
        .map(|c| {
            let mut attachments = Vec::new();
            let primary =
                resolve_slot(registry, &installed, &c.primary, &mut attachments, &mut skipped);
            let secondary =
                resolve_slot(registry, &installed, &c.secondary, &mut attachments, &mut skipped);
            let lethal =
                resolve_slot(registry, &installed, &c.lethal, &mut attachments, &mut skipped);
            let tactical =
                resolve_slot(registry, &installed, &c.tactical, &mut attachments, &mut skipped);
            for a in &c.attachments {
                attachments.push(resolve_slot(
                    registry,
                    &installed,
                    a,
                    &mut Vec::new(),
                    &mut skipped,
                ));
            }
            ClassRequest { primary, secondary, lethal, tactical, attachments, camo: c.camo.clone() }
        })
        .collect();
    if !skipped.is_empty() {
        // Una sola riga per piano (i preset ripetono le stesse armi molte volte).
        diag::info!(
            World,
            "selective weapons: {} preset weapon(s) skipped, namespace not installed: {:?}",
            skipped.len(),
            skipped
        );
    }
    let mut extra: Vec<String> = Vec::new();
    for raw in extra_always {
        // Un default assente dal registro e ignorato (non e un errore come per le classi).
        extra.extend(resolve_names(registry, raw));
    }
    let base = closure(registry, &classes, &extra)?;
    let mut variants: BTreeSet<String> = BTreeSet::new();
    for weapon in &base.weapons {
        // La chiave e `<ns>:<nome>`: il prefisso include il namespace, niente varianti incrociate.
        let stem = weapon.strip_suffix("_mp").unwrap_or(weapon);
        let prefix = format!("{stem}_");
        for (_, deps) in registry.iter() {
            if deps.weapon != *weapon && deps.weapon.starts_with(&prefix) {
                variants.insert(deps.weapon.clone());
            }
        }
    }
    let mut all_classes = classes;
    all_classes.push(ClassRequest {
        attachments: variants.into_iter().collect(),
        ..ClassRequest::default()
    });
    let n = all_classes.len() - 1;
    let closure = closure_with(registry, &all_classes, &extra)?;
    Ok(Selection { digest: selection_digest(&closure), closure, classes: n })
}

fn closure_with(
    registry: &WeaponRegistry,
    classes: &[ClassRequest],
    extra: &[String],
) -> Result<WeaponClosure, ClosureError> {
    closure(registry, classes, extra)
}

/// Prontezza per id CATALOGO: pronta se l'arma e nella closure (Residency non serve:
/// con il filtro in cattura le risorse della closure sono residenti dopo il walk).
pub fn readiness_for_selection(catalog: &WeaponCatalog, selection: &Selection) -> Vec<bool> {
    let mut out = vec![true; catalog.len() + 1];
    for index in 1..=catalog.len() as u32 {
        if let Some(key) = catalog_key(catalog, index) {
            out[index as usize] = selection.closure.weapons.contains(&key);
        }
    }
    out
}

/// Registro dipendenze dell'ultimo common set completo (shell common): serve a costruire
/// la closure del match. Nessun registro => nessuna selezione => precaricamento completo.
static REGISTRY: Mutex<Option<Arc<WeaponRegistry>>> = Mutex::new(None);
/// Classi aggiuntive (host/giocatore) dichiarate dal boot prima del walk.
static EXTRA_CLASSES: Mutex<Vec<ClassRequest>> = Mutex::new(Vec::new());
/// Selezione del walk in corso / dell'ultimo common set selettivo.
static ACTIVE: Mutex<Option<Arc<Selection>>> = Mutex::new(None);
/// Ultimo piano costruito e gli ingressi da cui deriva. Una partita chiede il piano piu volte
/// (chiave della cache, mappa residente): con gli stessi ingressi la risposta deve essere la
/// stessa, senza ricostruire, senza loggare di nuovo e senza sovrascrivere `ACTIVE`.
static LAST_PLAN: Mutex<Option<(u64, Option<Arc<Selection>>)>> = Mutex::new(None);

/// Firma degli ingressi del piano: registro (nomi), classi dichiarate dall'host, classi dei bot.
fn plan_signature(registry: &WeaponRegistry, classes: &[ClassRequest]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (_, deps) in registry.iter() {
        // Names AND the models/animations they pull in: a catalog that keeps the names but
        // changes what a weapon needs must not reuse a stale plan.
        deps.weapon.hash(&mut hasher);
        deps.gun_xmodels.hash(&mut hasher);
        deps.hand_xmodels.hash(&mut hasher);
        deps.world_xmodels.hash(&mut hasher);
        deps.projectile_xmodels.hash(&mut hasher);
        deps.xanims.hash(&mut hasher);
    }
    for class in classes {
        format!("{class:?}").hash(&mut hasher);
    }
    hasher.finish()
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

pub fn remember_registry(catalog: &WeaponCatalog) {
    let registry = registry_from_catalog(catalog);
    *lock(&REGISTRY) = Some(Arc::new(registry));
}

/// Il boot dichiara le classi del giocatore/host da rendere residenti.
pub fn set_extra_classes(classes: Vec<ClassRequest>) {
    *lock(&EXTRA_CLASSES) = classes;
    *lock(&EXTRA_DECLARED) = true;
}

/// Il boot ha dichiarato le classi host (anche se vuote). Senza dichiarazione la
/// selezione NON si attiva: il giocatore resterebbe senza le proprie armi.
static EXTRA_DECLARED: Mutex<bool> = Mutex::new(false);

/// Classi host/giocatore -> richieste di closure. Gli attachment sono codici (non nomi di
/// arma): le varianti sono coperte dal prefisso `<stem>_` in `build_selection`.
pub fn class_requests_from_host(slots: &[frame::HostClassSlot]) -> Vec<ClassRequest> {
    slots
        .iter()
        .map(|s| ClassRequest {
            primary: s.primary.clone(),
            secondary: s.secondary.clone(),
            lethal: s.lethal.clone(),
            tactical: s.tactical.clone(),
            ..ClassRequest::default()
        })
        .collect()
}

/// Dichiara le classi host (chiamato dal sync di HostClassLoadouts).
pub fn declare_host_classes(slots: &[frame::HostClassSlot]) {
    set_extra_classes(class_requests_from_host(slots));
}

/// Le classi del giocatore sono nella closure? Err = motivo del fallback.
pub fn verify_host_classes(
    registry: &WeaponRegistry,
    host: &[ClassRequest],
    selection: &Selection,
) -> Result<(), String> {
    if host.is_empty() {
        return Err("host class list is empty".to_owned());
    }
    let installed = installed_namespaces(registry);
    for c in host {
        for raw in [&c.primary, &c.secondary, &c.lethal, &c.tactical] {
            if raw.is_empty() {
                continue;
            }
            let names = resolve_names(registry, raw);
            // Un'arma di un namespace non installato non si puo equipaggiare comunque.
            if names.is_empty() && uninstalled_namespace(&installed, raw).is_some() {
                continue;
            }
            if names.is_empty() || !names.iter().all(|n| selection.closure.weapons.contains(n)) {
                return Err(format!("host class weapon {raw:?} not in closure"));
            }
        }
    }
    Ok(())
}

pub fn class_request_from_preset(p: &frame::ClassPreset) -> ClassRequest {
    ClassRequest {
        primary: p.primary.to_owned(),
        secondary: p.secondary.to_owned(),
        lethal: p.lethal.to_owned(),
        tactical: p.tactical.to_owned(),
        ..ClassRequest::default()
    }
}

/// Decide la selezione per il prossimo match. None = precaricamento completo
/// (modo non attivo, oppure FALLBACK esplicito loggato).
pub fn plan_selection() -> Option<Arc<Selection>> {
    let mode = crate::weapon_residency::selective_mode_from_env();
    let active = crate::weapon_residency::selective_active_from_env();
    if !active {
        diag::info!(World, "selective weapons: mode={mode:?} active=false classes=0 weapons=0 xmodels=0");
        *lock(&ACTIVE) = None;
        return None;
    }
    let Some(registry) = lock(&REGISTRY).clone() else {
        diag::warn!(
            World,
            "selective weapons FAILED -> full preload: no weapon registry yet (first common walk)"
        );
        *lock(&ACTIVE) = None;
        return None;
    };
    if !*lock(&EXTRA_DECLARED) {
        diag::warn!(
            World,
            "selective weapons FAILED -> full preload: host classes not declared before key decision"
        );
        *lock(&ACTIVE) = None;
        return None;
    }
    let host: Vec<ClassRequest> = lock(&EXTRA_CLASSES).clone();
    let mut classes: Vec<ClassRequest> =
        frame::showcase_classes().iter().map(class_request_from_preset).collect();
    classes.extend(host.iter().cloned());
    let signature = plan_signature(&registry, &classes);
    if let Some((last, cached)) = lock(&LAST_PLAN).clone()
        && last == signature
    {
        // Stessi ingressi: stesso piano (e stesso digest), nessun log duplicato.
        *lock(&ACTIVE) = cached.clone();
        return cached;
    }
    let extra_always = default_extra_always(&registry);
    let built = build_selection(&registry, &classes, &extra_always).and_then(|sel| {
        verify_host_classes(&registry, &host, &sel)
            .map(|()| sel)
            .map_err(ClosureError::MissingWeapon)
    });
    match built {
        Ok(selection) => {
            diag::info!(
                World,
                "selective weapons: mode={mode:?} active=true classes={} weapons={} xmodels={}",
                selection.classes,
                selection.closure.weapons.len(),
                selection.closure.xmodels.len()
            );
            let selection = Arc::new(selection);
            *lock(&ACTIVE) = Some(Arc::clone(&selection));
            *lock(&LAST_PLAN) = Some((signature, Some(Arc::clone(&selection))));
            Some(selection)
        }
        Err(error) => {
            diag::warn!(World, "selective weapons FAILED -> full preload: {error}");
            *lock(&ACTIVE) = None;
            *lock(&LAST_PLAN) = Some((signature, None));
            None
        }
    }
}

/// Closure del walk del common set in corso (impostata da prepare_common secondo la chiave).
static WALK: Mutex<Option<WeaponClosure>> = Mutex::new(None);

pub(crate) fn set_walk_closure(digest: Option<u64>) {
    let closure = digest.and_then(|d| {
        lock(&ACTIVE).as_ref().filter(|s| s.digest == d).map(|s| s.closure.clone())
    });
    if digest.is_some() && closure.is_none() {
        diag::warn!(
            World,
            "selective weapons FAILED -> full preload: selection for digest {:016x} no longer available",
            digest.unwrap_or_default()
        );
    }
    *lock(&WALK) = closure;
}

pub(crate) fn walk_closure() -> Option<WeaponClosure> {
    lock(&WALK).clone()
}

/// WeaponReadiness per il builder sim: active=false se nessuna selezione e in vigore.
pub fn current_readiness(catalog: &WeaponCatalog) -> (Vec<bool>, bool) {
    match lock(&ACTIVE).as_ref() {
        Some(selection) => (readiness_for_selection(catalog, selection), true),
        None => (Vec::new(), false),
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    use crate::weapon_residency::closure::test_util::{class, registry};

    #[test]
    fn digest_stable_and_set_dependent() {
        let reg = registry(&["m4", "ak", "usp"]);
        let a = build_selection(&reg, &[class("m4", "usp")], &[]).unwrap();
        let b = build_selection(&reg, &[class("m4", "usp")], &[]).unwrap();
        let c = build_selection(&reg, &[class("ak", "usp")], &[]).unwrap();
        assert_eq!(a.digest, b.digest);
        assert_ne!(a.digest, c.digest);
        assert!(a.closure.weapons.contains("m4"));
    }

    #[test]
    fn unknown_weapon_is_error() {
        let reg = registry(&["m4"]);
        assert!(build_selection(&reg, &[class("nope", "")], &[]).is_err());
    }

    #[test]
    fn host_classes_in_closure_and_change_digest() {
        let reg = registry(&["m4", "ak", "usp", "knife"]);
        let h1 = vec![class("m4", "usp")];
        let h2 = vec![class("ak", "usp")];
        let a = build_selection(&reg, &h1, &[]).unwrap();
        let b = build_selection(&reg, &h2, &[]).unwrap();
        assert!(a.closure.weapons.contains("m4"));
        assert!(b.closure.weapons.contains("ak"));
        assert!(!a.closure.weapons.contains("ak"));
        assert_ne!(a.digest, b.digest);
        assert!(verify_host_classes(&reg, &h1, &a).is_ok());
        assert!(verify_host_classes(&reg, &h2, &a).is_err());
        assert!(verify_host_classes(&reg, &[], &a).is_err());
    }

    #[test]
    fn preset_names_resolve_by_namespace_and_mp() {
        // Stesso nome base in due namespace: nessuna confusione.
        let reg = registry(&["t5:spectre", "iw4:spectre", "iw4:semtex", "iw5:iw5_ksg", "t5:knife_ballistic"]);
        let c = class("t5:weapon/spectre_mp", "t5:weapon/knife_ballistic_mp");
        let mut lethal = c.clone();
        lethal.lethal = "iw4:weapon/semtex_mp".into();
        lethal.primary = "iw5:weapon/iw5_ksg_mp".into();
        let sel = build_selection(&reg, &[c, lethal], &[]).unwrap();
        assert!(sel.closure.weapons.contains("t5:spectre"));
        assert!(!sel.closure.weapons.contains("iw4:spectre"));
        assert!(sel.closure.weapons.contains("iw4:semtex"));
        assert!(sel.closure.weapons.contains("iw5:iw5_ksg"));
        // Non qualificato con _mp: tutti i namespace col nome base (superset).
        let sel = build_selection(&reg, &[class("spectre_mp", "")], &[]).unwrap();
        assert!(sel.closure.weapons.contains("t5:spectre"));
        assert!(sel.closure.weapons.contains("iw4:spectre"));
        // Namespace sbagliato = errore esplicito, mai l'arma di un altro namespace.
        assert!(build_selection(&reg, &[class("iw5:weapon/spectre_mp", "")], &[]).is_err());
    }

    #[test]
    fn plan_signature_depends_on_registry_and_classes_not_on_call_count() {
        let reg = registry(&["iw4:m4a1", "iw4:usp"]);
        let a = vec![class("iw4:weapon/m4a1_mp", "iw4:weapon/usp_mp")];
        let b = vec![class("iw4:weapon/usp_mp", "")];
        assert_eq!(plan_signature(&reg, &a), plan_signature(&reg, &a), "same inputs, same signature");
        assert_ne!(plan_signature(&reg, &a), plan_signature(&reg, &b), "a class change must replan");
        let other = registry(&["iw4:m4a1", "iw4:usp", "iw4:ak47"]);
        assert_ne!(plan_signature(&reg, &a), plan_signature(&other, &a), "a new catalog must replan");
    }

    #[test]
    fn system_weapons_are_always_resident_when_the_catalog_has_them() {
        let reg = registry(&[
            "iw4:knife", "iw4:airdrop_marker", "iw4:flare", "iw4:riotshield", "iw4:stinger",
            "iw4:javelin", "iw4:killstreak_predator", "iw4:m4a1",
        ]);
        let always = default_extra_always(&reg);
        for must in ["iw4:knife", "iw4:airdrop_marker", "iw4:flare", "iw4:riotshield", "iw4:stinger", "iw4:javelin", "iw4:killstreak_predator"] {
            assert!(always.iter().any(|w| w == must), "{must} must always be resident: {always:?}");
        }
        assert!(!always.iter().any(|w| w == "iw4:m4a1"), "class weapons are not forced: {always:?}");
        // Names missing from the catalog are skipped, not an error.
        let sel = build_selection(&reg, &[class("iw4:weapon/m4a1_mp", "")], &always).unwrap();
        assert!(sel.closure.weapons.contains("iw4:airdrop_marker"));
    }

    #[test]
    fn uninstalled_namespace_is_skipped_but_unknown_in_installed_is_error() {
        // Catalogo reale: solo IW4. I preset citano anche t5/t6/iw5 (zone non installate).
        let reg = registry(&["iw4:m4a1", "iw4:usp", "iw4:semtex"]);
        let mixed = class("t5:weapon/spectre_mp", "iw4:weapon/usp_mp");
        let mut other = class("iw4:weapon/m4a1_mp", "t6:weapon/fnp45_mp");
        other.lethal = "iw4:weapon/semtex_mp".into();
        other.tactical = "iw5:weapon/xm25_mp".into();
        let sel = build_selection(&reg, &[mixed, other], &[]).expect("non installed is skipped");
        assert!(sel.closure.weapons.contains("iw4:usp"));
        assert!(sel.closure.weapons.contains("iw4:m4a1"));
        assert!(sel.closure.weapons.contains("iw4:semtex"));
        assert_eq!(sel.closure.weapons.len(), 3);
        // Arma ignota in un namespace PRESENTE: errore visibile, mai saltata in silenzio.
        assert!(build_selection(&reg, &[class("iw4:weapon/nope_mp", "")], &[]).is_err());
        // Classe host con arma di namespace non installato: non blocca la selezione.
        let host = vec![class("iw4:weapon/m4a1_mp", "t5:weapon/cz75_mp")];
        let sel = build_selection(&reg, &host, &[]).unwrap();
        assert!(verify_host_classes(&reg, &host, &sel).is_ok());
        // ...ma un'arma host ignota in un namespace presente si.
        let bad = vec![class("iw4:weapon/nope_mp", "")];
        assert!(verify_host_classes(&reg, &bad, &sel).is_err());
    }

    #[test]
    fn showcase_presets_resolve_against_matching_registry() {
        use std::collections::BTreeSet;
        let mut keys: BTreeSet<String> = BTreeSet::new();
        for p in frame::showcase_classes() {
            for raw in [p.primary, p.secondary, p.lethal, p.tactical] {
                if raw.is_empty() {
                    continue;
                }
                let key = match raw.split_once(':') {
                    Some((ns, rest)) => format!("{ns}:{}", normalize_name(rest.rsplit('/').next().unwrap())),
                    None => normalize_name(raw),
                };
                keys.insert(key);
            }
        }
        let names: Vec<&str> = keys.iter().map(String::as_str).collect();
        let reg = registry(&names);
        let classes: Vec<ClassRequest> = frame::showcase_classes().iter().map(class_request_from_preset).collect();
        let sel = build_selection(&reg, &classes, &[]).expect("tutti i preset si risolvono");
        for k in &keys {
            assert!(sel.closure.weapons.contains(k), "{k}");
        }
    }
}
