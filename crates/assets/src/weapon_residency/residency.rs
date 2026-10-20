//! Residency state machine: dedup, pins, generations, per-tick promotion budget.

use std::collections::BTreeMap;
use std::fmt;

use super::closure::{closure, ClassRequest, ClosureError, ResourceKind, WeaponRegistry};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owner {
    Player,
    Bot(u32),
    Pickup,
    Killcam,
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Owner::Player => write!(f, "player"),
            Owner::Bot(i) => write!(f, "bot{i}"),
            Owner::Pickup => write!(f, "pickup"),
            Owner::Killcam => write!(f, "killcam"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResKey {
    pub namespace: String,
    pub kind: ResourceKind,
    pub name: String,
}

impl ResKey {
    pub fn new(namespace: &str, kind: ResourceKind, name: &str) -> Self {
        Self { namespace: namespace.to_string(), kind, name: name.to_string() }
    }
}

impl fmt::Display for ResKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{:?}/{}", self.namespace, self.kind, self.name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResState {
    Requested,
    Preparing,
    Ready,
    Active,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestError {
    /// Generation older than the current one: ignored, nothing changed.
    Stale { request: u64, current: u64 },
    MissingWeapon(String),
    Failed { key: ResKey, reason: String },
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Stale { request, current } => {
                write!(f, "stale generation {request} (current {current})")
            }
            RequestError::MissingWeapon(n) => write!(f, "unknown weapon {n:?}"),
            RequestError::Failed { key, reason } => write!(f, "resource {key} failed: {reason}"),
        }
    }
}

impl std::error::Error for RequestError {}

impl From<ClosureError> for RequestError {
    fn from(e: ClosureError) -> Self {
        match e {
            ClosureError::MissingWeapon(n) => RequestError::MissingWeapon(n),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResidencyEvent {
    Requested { key: ResKey, owner: Owner },
    Reused { key: ResKey, owner: Owner },
    Prepared { key: ResKey },
    /// `removed`: last pin dropped, entry gone; otherwise other pins remain.
    Released { key: ResKey, owner: Owner, removed: bool },
    Failed { key: ResKey, reason: String },
    /// Ready promotion postponed by the per-tick budget.
    Deferred { key: ResKey },
}

impl ResidencyEvent {
    pub fn log(&self) {
        match self {
            ResidencyEvent::Requested { key, owner } => {
                diag::info!(World, "weapons: requested {key} owner={owner}")
            }
            ResidencyEvent::Reused { key, owner } => {
                diag::info!(World, "weapons: reused {key} owner={owner}")
            }
            ResidencyEvent::Prepared { key } => diag::info!(World, "weapons: prepared {key}"),
            ResidencyEvent::Released { key, owner, removed } => {
                diag::info!(World, "weapons: released {key} owner={owner} removed={removed}")
            }
            ResidencyEvent::Failed { key, reason } => {
                diag::warn!(World, "weapons: failed {key}: {reason}")
            }
            ResidencyEvent::Deferred { key } => {
                diag::info!(World, "weapons: deferred {key} (frame budget)")
            }
        }
    }
}

#[derive(Debug)]
struct Entry {
    state: ResState,
    pins: BTreeMap<Owner, u32>,
    /// Preparation finished OK, waiting for a budget slot to become Ready.
    finished: bool,
}

#[derive(Debug)]
pub struct Residency {
    namespace: String,
    generation: u64,
    budget_per_tick: usize,
    entries: BTreeMap<ResKey, Entry>,
    owner_keys: BTreeMap<Owner, Vec<ResKey>>,
}

impl Residency {
    pub fn new(namespace: &str, budget_per_tick: usize) -> Self {
        Self {
            namespace: namespace.to_string(),
            generation: 0,
            budget_per_tick: budget_per_tick.max(1),
            entries: BTreeMap::new(),
            owner_keys: BTreeMap::new(),
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn state(&self, key: &ResKey) -> Option<&ResState> {
        self.entries.get(key).map(|e| &e.state)
    }

    pub fn pin_count(&self, key: &ResKey) -> u32 {
        self.entries.get(key).map_or(0, |e| e.pins.values().sum())
    }

    pub fn is_ready(&self, key: &ResKey) -> bool {
        matches!(self.state(key), Some(ResState::Ready | ResState::Active))
    }

    pub fn key(&self, kind: ResourceKind, name: &str) -> ResKey {
        ResKey::new(&self.namespace, kind, name)
    }

    pub fn failed(&self) -> Vec<(ResKey, String)> {
        self.entries
            .iter()
            .filter_map(|(k, e)| match &e.state {
                ResState::Failed(r) => Some((k.clone(), r.clone())),
                _ => None,
            })
            .collect()
    }

    /// Start a newer generation: every preparation/pin of the old one is cancelled.
    /// A generation <= current is a no-op.
    pub fn begin_generation(&mut self, generation: u64) -> Vec<ResidencyEvent> {
        if generation <= self.generation {
            return Vec::new();
        }
        let mut ev = Vec::new();
        for (key, e) in std::mem::take(&mut self.entries) {
            for owner in e.pins.keys() {
                ev.push(ResidencyEvent::Released { key: key.clone(), owner: *owner, removed: true });
            }
        }
        self.owner_keys.clear();
        self.generation = generation;
        ev
    }

    /// Single entry point: (re)declare the class an owner needs. Replaces whatever the
    /// owner requested before (changed class never leaves the old one pinned).
    pub fn request_class(
        &mut self,
        registry: &WeaponRegistry,
        owner: Owner,
        class: &ClassRequest,
        generation: u64,
    ) -> Result<Vec<ResidencyEvent>, RequestError> {
        if generation < self.generation {
            return Err(RequestError::Stale { request: generation, current: self.generation });
        }
        let mut events = self.begin_generation(generation);
        let clo = closure(registry, std::slice::from_ref(class), &[])?;
        let keys: Vec<ResKey> = clo
            .resources()
            .into_iter()
            .map(|(k, n)| ResKey::new(&self.namespace, k, &n))
            .collect();
        for k in &keys {
            if let Some(Entry { state: ResState::Failed(reason), .. }) = self.entries.get(k) {
                return Err(RequestError::Failed { key: k.clone(), reason: reason.clone() });
            }
        }
        // Pin the new set first so shared resources never drop to zero.
        for k in &keys {
            let existed = self.entries.contains_key(k);
            let e = self.entries.entry(k.clone()).or_insert_with(|| Entry {
                state: ResState::Requested,
                pins: BTreeMap::new(),
                finished: false,
            });
            *e.pins.entry(owner).or_insert(0) += 1;
            events.push(if existed {
                ResidencyEvent::Reused { key: k.clone(), owner }
            } else {
                ResidencyEvent::Requested { key: k.clone(), owner }
            });
        }
        if let Some(old) = self.owner_keys.insert(owner, keys) {
            for k in old {
                self.unpin(&k, owner, &mut events);
            }
        }
        Ok(events)
    }

    /// Drop everything an owner holds (bot left, killcam ended, ...).
    pub fn release_owner(&mut self, owner: Owner) -> Vec<ResidencyEvent> {
        let mut events = Vec::new();
        if let Some(keys) = self.owner_keys.remove(&owner) {
            for k in keys {
                self.unpin(&k, owner, &mut events);
            }
        }
        events
    }

    fn unpin(&mut self, key: &ResKey, owner: Owner, events: &mut Vec<ResidencyEvent>) {
        let Some(e) = self.entries.get_mut(key) else { return };
        if let Some(n) = e.pins.get_mut(&owner) {
            *n -= 1;
            if *n == 0 {
                e.pins.remove(&owner);
            }
        }
        let removed = e.pins.is_empty();
        if removed {
            self.entries.remove(key);
        }
        events.push(ResidencyEvent::Released { key: key.clone(), owner, removed });
    }

    /// Requested -> Preparing. Returns the keys the caller must now prepare (one job each).
    pub fn drain_requested(&mut self) -> Vec<ResKey> {
        let mut out = Vec::new();
        for (k, e) in &mut self.entries {
            if e.state == ResState::Requested {
                e.state = ResState::Preparing;
                out.push(k.clone());
            }
        }
        out
    }

    /// Report a preparation result. Unknown/cancelled keys (stale job) are ignored.
    pub fn complete(&mut self, key: &ResKey, result: Result<(), String>) -> Option<ResidencyEvent> {
        let e = self.entries.get_mut(key)?;
        if e.state != ResState::Preparing {
            return None;
        }
        match result {
            Ok(()) => {
                e.finished = true;
                None
            }
            Err(reason) => {
                e.state = ResState::Failed(reason.clone());
                Some(ResidencyEvent::Failed { key: key.clone(), reason })
            }
        }
    }

    /// Promote at most `budget_per_tick` finished resources Preparing -> Ready.
    pub fn tick(&mut self) -> Vec<ResidencyEvent> {
        let mut events = Vec::new();
        let mut left = self.budget_per_tick;
        for (k, e) in &mut self.entries {
            if e.state == ResState::Preparing && e.finished {
                if left == 0 {
                    events.push(ResidencyEvent::Deferred { key: k.clone() });
                } else {
                    left -= 1;
                    e.state = ResState::Ready;
                    events.push(ResidencyEvent::Prepared { key: k.clone() });
                }
            }
        }
        events
    }

    /// Ready -> Active (first use). False if not Ready.
    pub fn activate(&mut self, key: &ResKey) -> bool {
        match self.entries.get_mut(key) {
            Some(e) if e.state == ResState::Ready => {
                e.state = ResState::Active;
                true
            }
            Some(e) => e.state == ResState::Active,
            None => false,
        }
    }

    /// Flat table indexed by weapon id: true when the weapon and all its deps are Ready/Active.
    pub fn ready_table(&self, registry: &WeaponRegistry) -> Vec<bool> {
        registry
            .iter()
            .map(|(_, d)| {
                let mut keys = vec![self.key(ResourceKind::Weapon, &d.weapon)];
                let c = closure(registry, &[], std::slice::from_ref(&d.weapon));
                if let Ok(c) = c {
                    keys.extend(c.resources().into_iter().map(|(k, n)| self.key(k, &n)));
                }
                keys.iter().all(|k| self.is_ready(k))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::closure::test_util::*;
    use super::*;

    fn run_all(r: &mut Residency) {
        for k in r.drain_requested() {
            r.complete(&k, Ok(()));
        }
        r.tick();
    }

    fn wkey(r: &Residency, n: &str) -> ResKey {
        r.key(ResourceKind::Weapon, n)
    }

    #[test]
    fn concurrent_requests_prepare_once() {
        let reg = registry(&["m4", "usp"]);
        let mut r = Residency::new("mp", 1000);
        r.request_class(&reg, Owner::Player, &class("m4", "usp"), 1).unwrap();
        let ev = r.request_class(&reg, Owner::Bot(1), &class("m4", "usp"), 1).unwrap();
        assert!(ev.iter().all(|e| matches!(e, ResidencyEvent::Reused { .. })));
        let jobs = r.drain_requested();
        let mut dedup = jobs.clone();
        dedup.dedup();
        assert_eq!(jobs, dedup);
        assert_eq!(jobs.iter().filter(|k| **k == wkey(&r, "m4")).count(), 1);
        assert!(r.drain_requested().is_empty(), "no second preparation");
        assert_eq!(r.pin_count(&wkey(&r, "m4")), 2);
    }

    #[test]
    fn class_change_unpins_old() {
        let reg = registry(&["m4", "ak", "usp"]);
        let mut r = Residency::new("mp", 1000);
        r.request_class(&reg, Owner::Player, &class("m4", "usp"), 1).unwrap();
        run_all(&mut r);
        assert!(r.is_ready(&wkey(&r, "m4")));
        r.request_class(&reg, Owner::Player, &class("ak", "usp"), 1).unwrap();
        assert_eq!(r.state(&wkey(&r, "m4")), None, "old class not left pinned");
        assert_eq!(r.pin_count(&wkey(&r, "usp")), 1);
        assert!(r.is_ready(&wkey(&r, "usp")), "shared resource kept ready");
        assert!(r.state(&wkey(&r, "ak")).is_some());
    }

    #[test]
    fn class_change_during_preparation_cancels_old() {
        let reg = registry(&["m4", "ak"]);
        let mut r = Residency::new("mp", 1000);
        r.request_class(&reg, Owner::Player, &class("m4", ""), 1).unwrap();
        let jobs = r.drain_requested();
        r.request_class(&reg, Owner::Player, &class("ak", ""), 1).unwrap();
        for k in &jobs {
            r.complete(k, Ok(()));
        }
        r.tick();
        assert!(r.state(&wkey(&r, "m4")).is_none());
        assert!(!r.is_ready(&wkey(&r, "m4")));
    }

    #[test]
    fn bot_added_mid_match_reuses_and_adds() {
        let reg = registry(&["m4", "ak"]);
        let mut r = Residency::new("mp", 1000);
        r.request_class(&reg, Owner::Player, &class("m4", ""), 1).unwrap();
        run_all(&mut r);
        r.request_class(&reg, Owner::Bot(7), &class("m4", "ak"), 1).unwrap();
        assert!(r.is_ready(&wkey(&r, "m4")));
        assert!(!r.is_ready(&wkey(&r, "ak")));
        assert!(r.drain_requested().iter().any(|k| k.name.starts_with("ak")));
        r.release_owner(Owner::Bot(7));
        assert!(r.is_ready(&wkey(&r, "m4")), "player still pins m4");
        assert!(r.state(&wkey(&r, "ak")).is_none());
    }

    #[test]
    fn unknown_weapon_and_failed_are_explicit() {
        let reg = registry(&["m4"]);
        let mut r = Residency::new("mp", 1000);
        assert_eq!(
            r.request_class(&reg, Owner::Player, &class("m4", "zzz"), 1),
            Err(RequestError::MissingWeapon("zzz".into()))
        );
        assert!(r.is_empty());
        r.request_class(&reg, Owner::Player, &class("m4", ""), 1).unwrap();
        let k = wkey(&r, "m4");
        r.drain_requested();
        let ev = r.complete(&k, Err("decode".into())).unwrap();
        assert!(matches!(ev, ResidencyEvent::Failed { .. }));
        assert_eq!(r.state(&k), Some(&ResState::Failed("decode".into())));
        assert!(!r.is_ready(&k));
        assert_eq!(r.failed().len(), 1);
        assert!(matches!(
            r.request_class(&reg, Owner::Bot(1), &class("m4", ""), 1),
            Err(RequestError::Failed { .. })
        ));
    }

    #[test]
    fn new_generation_cancels_and_stale_ignored() {
        let reg = registry(&["m4", "ak"]);
        let mut r = Residency::new("mp", 1000);
        r.request_class(&reg, Owner::Player, &class("m4", ""), 1).unwrap();
        let jobs = r.drain_requested();
        r.request_class(&reg, Owner::Player, &class("ak", ""), 2).unwrap();
        assert_eq!(r.generation(), 2);
        assert!(r.state(&wkey(&r, "m4")).is_none());
        for k in &jobs {
            assert!(r.complete(k, Ok(())).is_none());
        }
        let before = r.len();
        assert_eq!(
            r.request_class(&reg, Owner::Player, &class("m4", ""), 1),
            Err(RequestError::Stale { request: 1, current: 2 })
        );
        assert_eq!(r.len(), before);
    }

    #[test]
    fn multi_pin_survives_first_release() {
        let reg = registry(&["m4"]);
        let mut r = Residency::new("mp", 1000);
        r.request_class(&reg, Owner::Player, &class("m4", ""), 1).unwrap();
        r.request_class(&reg, Owner::Killcam, &class("m4", ""), 1).unwrap();
        run_all(&mut r);
        let k = wkey(&r, "m4");
        let ev = r.release_owner(Owner::Killcam);
        assert!(ev.iter().all(|e| matches!(e, ResidencyEvent::Released { removed: false, .. })));
        assert!(r.is_ready(&k));
        r.release_owner(Owner::Player);
        assert!(r.state(&k).is_none());
    }

    #[test]
    fn budget_limits_promotions_per_tick() {
        let reg = registry(&["m4"]);
        let mut r = Residency::new("mp", 2);
        r.request_class(&reg, Owner::Player, &class("m4", ""), 1).unwrap();
        let total = r.len();
        assert!(total > 2);
        for k in r.drain_requested() {
            r.complete(&k, Ok(()));
        }
        let ev = r.tick();
        assert_eq!(ev.iter().filter(|e| matches!(e, ResidencyEvent::Prepared { .. })).count(), 2);
        assert!(ev.iter().any(|e| matches!(e, ResidencyEvent::Deferred { .. })));
        let mut ticks = 1;
        while !reg.iter().all(|(i, _)| r.ready_table(&reg)[i as usize]) {
            r.tick();
            ticks += 1;
            assert!(ticks < 20);
        }
    }

    #[test]
    fn ready_table_matches_ids() {
        let reg = registry(&["m4", "ak", "usp"]);
        let mut r = Residency::new("mp", 1000);
        assert_eq!(r.ready_table(&reg), vec![false; 3]);
        r.request_class(&reg, Owner::Player, &class("ak", ""), 1).unwrap();
        run_all(&mut r);
        assert_eq!(r.ready_table(&reg), vec![false, true, false]);
        let k = wkey(&r, "ak");
        assert!(r.activate(&k));
        assert_eq!(r.state(&k), Some(&ResState::Active));
        assert_eq!(r.ready_table(&reg), vec![false, true, false]);
    }
}
