use bevy::prelude::Resource;
use sim::{ClientId, LifeSequence};

use crate::controller::HostController;

/// Most participants in one lobby or match, humans and bots together. This is the only
/// capacity limit; the lobby, the menus and the roster all derive theirs from it.
pub const MAX_PARTICIPANTS: u32 = 18;

/// Most bots a match can hold: at least one human is always playing.
pub const MAX_HOST_BOTS: u32 = MAX_PARTICIPANTS - 1;

/// Bots that fit in a lobby holding `humans` people; the host always counts as one.
pub fn bot_room_for(humans: u32) -> u32 {
    MAX_PARTICIPANTS.saturating_sub(humans.max(1))
}

const BOT_NAMES: &[&str] = &[
    "xXsn1perXx",
    "xX_QuickScope_X",
    "xXNoScopeXx",
    "Xx360NoScopexX",
    "xXDarkLord69Xx",
    "xXPwnageXx",
    "xXMLG_ProXx",
    "xXTrickshotXx",
    "xXHeadshotXx",
    "xXDeadeyeXx",
    "xXShadowXx",
    "xXKillaXx",
    "ImbaPwnz",
    "Mt_Dew_Dad",
    "DoritoDan",
    "Snip3rWolf",
    "FaZe_Bob",
    "OptiMLG",
    "MLGsnoopy",
    "iHaxPro",
    "Ligma",
    "SugmaBalls",
    "Sawcondese",
    "Joe_Mama",
    "Bofa_Dad",
    "Deez_Nuts",
    "ImBadAtThis",
    "Yeet",
    "GetRekt",
    "Rekt_u",
    "420BlazeIt",
    "xX420Xx",
    "WeedLord",
    "NoobSlayer",
    "n00bTube",
    "Pwn3d",
    "KnifeOnly",
    "NoSc0pe",
    "OneShotOne",
    "Hardscoper",
    "AimbotAndy",
    "NotAHacker",
    "ModdedLobby",
    "RIP_Rust",
    "HaloReach",
    "Mom_Get_Cam",
    "TheBigMan",
    "ChairBreaker",
    "Rage_Quit",
    "Lag_Switch",
    "QuickyBoy",
    "RustPwner",
    "Sniper_Boi",
    "ProGamerMove",
    "GG_EZ",
    "ez_clap",
    "OopsDied",
    "WheresMyTeam",
    "Camper_Carl",
    "AFK_Andy",
    "ChatGPTDeezNuts",
    "OnlyRAMs",
    "CtrlAltDefeat",
    "AltF4YourMom",
    "404NoBitches",
    "StackOverHoe",
    "SudoEatMyAss",
    "BlueScreenBalls",
    "CallOfBooty",
    "CumAndConquer",
    "WankOfWarcraft",
    "AssCreed",
    "RedDeadErection",
    "EldenSchlong",
    "NoScopeNoBitches",
    "PrematureReload",
    "TacticalTeabag",
    "CampAndCope",
    "ObiWanCanBlowMe",
    "DarthVibrator",
    "HarryPooter",
    "LordOfTheDings",
    "ShrekOnMyFace",
    "WalterWank",
    "BetterCallBalls",
    "GandalfTheGay",
    "FrodoTeaBaggins",
    "WillyWanker",
    "MrLeast",
    "BeastDeezNuts",
    "SubscribeOrDie",
    "RaidShadowBalls",
    "NordVPNis",
    "FyreFestVIP",
    "CryptoRugDaddy",
    "OnlyScams",
    "ApologyUkulele",
    "ColossalSkillIssue",
    "SamAltF4",
    "ClippyGotHard",
    "DeepSeekDeez",
    "GrokTuah",
    "GitPushMyAss",
    "HotfixMyBalls",
    "SeggsFault",
    "AcceptMyCookies",
    "MicroSoftDick",
    "RebootyCall",
    "DukeNutted",
    "MasterQueef",
    "SolidSnakeOil",
    "MetalGearSuck",
    "AssEffect",
    "GirthOfWar",
    "MortalCumback",
    "FapSouls",
    "SekiroNoBitches",
    "Battlefield69",
    "PayToWhine",
    "FlashbangMyAss",
    "CountFapula",
    "Dumbledong",
    "HagridGotHard",
    "Voldemortgage",
    "ThanosDidHalf",
    "TheWankingDead",
    "BreakingBalls",
    "StrangerDongs",
    "JohnWank",
    "JackOffSparrow",
    "HawkTuah404",
    "MrBreast",
    "PewDiePieChart",
    "iShowDebt",
    "SkibidiTaxFraud",
    "SigmaBalls",
    "RizzInPeace",
    "SponsoredByDebt",
    "TrustMeBroCoin",
    "LinkInMyBussy",
    "YourMomsDuo",
    "MyWifeLeft",
    "DivorcedByPing",
    "BornToRespawn",
    "UnpaidHitman",
    "SendDudes",
    "TeabagReceipt",
    "OopsWrongHole",
];

// The protocol stores 16 bytes including NUL. All approved names are ASCII.
// Keep the full pool above, but use the same explicit 15-byte display name in
// the lobby and authoritative game state (including the two longer names).
const BOT_NAME_BYTES: usize = 15;
/// MW2 ranks are indexed from 0 (level 1) through 69 (level 70). From the original game;
/// a rank row the loaded tables do not have simply shows no emblem.
pub const MAX_BOT_RANK: i32 = 69;
/// MW2 exposes ten prestige emblems in addition to no prestige.
pub const MAX_BOT_PRESTIGE: i32 = 10;

fn display_name(name: &'static str) -> &'static str {
    &name[..name.len().min(BOT_NAME_BYTES)]
}

fn shuffle_names(names: &mut [&'static str], mut random: impl FnMut() -> u64) {
    for i in (1..names.len()).rev() {
        // Rejection sampling avoids modulo bias.
        let bound = (i + 1) as u64;
        let threshold = bound.wrapping_neg() % bound;
        let value = loop {
            let value = random();
            if value >= threshold {
                break value;
            }
        };
        names.swap(i, (value % bound) as usize);
    }
}

/// Joke clan tags. A tag goes in front of a name only when `[TAG]name` still fits the
/// 15 bytes the protocol allows; the name itself is never cut to make room.
const CLAN_TAGS: &[&str] = &[
    "420", "69", "GG", "EZ", "TTV", "MLG", "FBI", "NSA", "CIA", "IRS", "DMV", "HR", "LOL",
    "LMAO", "WTF", "OMG", "BRB", "AFK", "RIP", "OOF", "BAN", "BOT", "SUS", "UWU", "NPC", "RNG",
    "LAG", "FPS", "KDA", "DAD", "MOM", "DUI", "ASS", "BUM", "FAP", "HOE", "DEEZ", "COPE",
    "BRUH", "SKIB", "RIZZ", "GOON", "SIGM", "NUT", "WANK", "BALZ", "TITS", "YOLO", "SWAG",
    "NOOB", "PWNT", "HAX", "TRY", "SKLL", "ISSU", "CAMP", "TBAG", "SCAM", "FOMO", "HODL",
    "ONLY", "SIMP", "KARN", "MEME", "EDGY", "DRIP", "BEEF", "LARP", "NERF", "BUFF", "GIT",
    "SUDO", "RTFM", "TODO", "NULL", "VOID", "404", "500", "WIFI", "5G", "VPN", "DNS", "RAM",
];

/// Probability (in tenths) that a name that has room for one gets a tag.
const TAG_CHANCE_TENTHS: u64 = 9;
/// `[IRS]` deliberately occurs more often than the other joke tags.
const IRS_TAG_WEIGHT: u64 = 8;

fn entropy_seed() -> u64 {
    let mut entropy = [0u8; 8];
    match getrandom::fill(&mut entropy) {
        Ok(()) => u64::from_le_bytes(entropy),
        Err(error) => {
            diag::warn!(
                Sim,
                "bots: name shuffle entropy unavailable: {error}; using counter fallback"
            );
            static FALLBACK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            FALLBACK
                .fetch_add(0x9e37_79b9, std::sync::atomic::Ordering::Relaxed)
                .wrapping_mul(0xd1b5_4a32_d192_ed03)
        }
    }
}

/// SplitMix64: independent from bot AI and account identity RNGs.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Each composed `[TAG]name` is leaked once and reused, so repeated rerolls cannot grow
/// memory beyond the number of distinct tag/name pairs.
fn intern(text: String) -> &'static str {
    static POOL: std::sync::Mutex<Option<std::collections::HashMap<String, &'static str>>> =
        std::sync::Mutex::new(None);
    let mut guard = POOL.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(Default::default);
    if let Some(found) = map.get(&text) {
        return found;
    }
    let leaked: &'static str = Box::leak(text.clone().into_boxed_str());
    map.insert(text, leaked);
    leaked
}

fn with_tag(name: &'static str, random: &mut impl FnMut() -> u64) -> &'static str {
    if random() % 10 >= TAG_CHANCE_TENTHS {
        return name;
    }
    let fitting: Vec<&str> = CLAN_TAGS
        .iter()
        .copied()
        .filter(|tag| tag.len() + 2 + name.len() <= BOT_NAME_BYTES)
        .collect();
    if fitting.is_empty() {
        return name;
    }
    let total_weight: u64 = fitting
        .iter()
        .map(|tag| if *tag == "IRS" { IRS_TAG_WEIGHT } else { 1 })
        .sum();
    let mut choice = random() % total_weight;
    let tag = fitting
        .into_iter()
        .find(|tag| {
            let weight = if *tag == "IRS" { IRS_TAG_WEIGHT } else { 1 };
            if choice < weight {
                true
            } else {
                choice -= weight;
                false
            }
        })
        .expect("a non-empty tag list has positive weight");
    intern(format!("[{tag}]{name}"))
}

/// An immutable identity drawn once for each lobby.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BotIdentity {
    pub name: &'static str,
    pub rank: i32,
    pub prestige: i32,
}

impl BotIdentity {
    pub const fn new(name: &'static str, rank: i32, prestige: i32) -> Self {
        Self {
            name,
            rank,
            prestige,
        }
    }
}

fn draw_rank_prestige(random: &mut impl FnMut() -> u64) -> (i32, i32) {
    let rank = (random() % (MAX_BOT_RANK as u64 + 1)) as i32;
    // Prestige resets the level to 1, so any rank can carry any prestige.
    let prestige = if random() % 10 < 6 {
        0
    } else {
        (random() % MAX_BOT_PRESTIGE as u64 + 1) as i32
    };
    (rank, prestige)
}

/// A fresh random order of the whole pool, each name with a matching rank and prestige.
fn draw_identities() -> Vec<BotIdentity> {
    let mut names: Vec<_> = BOT_NAMES.iter().map(|name| display_name(name)).collect();
    let mut state = entropy_seed();
    shuffle_names(&mut names, || splitmix(&mut state));
    names
        .into_iter()
        .map(|name| {
            let name = with_tag(name, &mut || splitmix(&mut state));
            let (rank, prestige) = draw_rank_prestige(&mut || splitmix(&mut state));
            BotIdentity::new(name, rank, prestige)
        })
        .collect()
}

/// The random name order of the current lobby, drawn on first use. Bot `n` takes the
/// `n`-th name, so changing the count never renames the bots that stay.
static DRAW: std::sync::Mutex<Option<Vec<BotIdentity>>> = std::sync::Mutex::new(None);

fn lock_draw() -> std::sync::MutexGuard<'static, Option<Vec<BotIdentity>>> {
    DRAW.lock().unwrap_or_else(|e| e.into_inner())
}

/// The names the lobby shows for `count` bots. They are a prefix of one order drawn per
/// lobby: adding or removing a bot, a lobby refresh or a map load keep the names, and the
/// match then takes exactly these names.
pub fn lobby_bot_names(count: u32) -> Vec<&'static str> {
    lock_draw()
        .get_or_insert_with(draw_identities)
        .iter()
        .map(|identity| identity.name)
        .take(count as usize)
        .collect()
}

/// Forget the current order; the next lobby draws its own. Only a new lobby calls this.
pub fn reset_lobby_names() {
    *lock_draw() = None;
}

/// The current order, drawn on first use. Bots added in a match (also by console
/// command) follow it without redrawing it.
fn match_identities() -> Vec<BotIdentity> {
    lock_draw().get_or_insert_with(draw_identities).clone()
}

#[derive(Resource, Debug, Default)]
pub struct BotAddQueue(pub Vec<BotAddRequest>);

#[derive(Debug)]
pub struct BotAddRequest {
    pub count: u32,
    pub dummy: bool,
}

impl BotAddQueue {
    pub fn push(&mut self, count: u32) {
        self.0.push(BotAddRequest {
            count: count.max(1),
            dummy: false,
        });
    }

    pub fn push_dummy(&mut self, count: u32) {
        self.0.push(BotAddRequest {
            count: count.max(1),
            dummy: true,
        });
    }

    pub fn drain(&mut self) -> Vec<BotAddRequest> {
        core::mem::take(&mut self.0)
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct BotHold(pub bool);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BotTpTarget {
    All,
    Id(ClientId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BotTpWhere {
    Absolute {
        origin: [f32; 3],
        yaw: Option<f32>,
        pitch: Option<f32>,
    },

    Above {
        height: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BotTpRequest {
    pub target: BotTpTarget,
    pub where_: BotTpWhere,
}

#[derive(Resource, Debug, Default)]
pub struct BotTpQueue(pub Vec<BotTpRequest>);

impl BotTpQueue {
    pub fn push(&mut self, request: BotTpRequest) {
        self.0.push(request);
    }

    pub fn drain(&mut self) -> Vec<BotTpRequest> {
        core::mem::take(&mut self.0)
    }
}

#[derive(Resource, Debug, Default)]
pub struct BotFireQueue(pub Vec<BotTpTarget>);

impl BotFireQueue {
    pub fn push(&mut self, target: BotTpTarget) {
        self.0.push(target);
    }

    pub fn drain(&mut self) -> Vec<BotTpTarget> {
        core::mem::take(&mut self.0)
    }
}

#[derive(Debug)]
pub struct BotSlot {
    pub id: ClientId,
    pub name: &'static str,
    pub rank: i32,
    pub prestige: i32,
    pub account: sim::AccountId,
    pub brain: Option<HostController>,
    pub joined: bool,
    pub class_picked_in: Option<LifeSequence>,
    pub class_picks: u32,
}

#[derive(Resource, Debug)]
pub struct BotRoster {
    pub bots: Vec<BotSlot>,
    pub next_client: u32,
    pub seed: u64,
}

impl Default for BotRoster {
    fn default() -> Self {
        Self {
            bots: Vec::new(),

            next_client: 1,
            seed: 0xb075_0001,
        }
    }
}

impl BotRoster {
    pub fn is_bot(&self, id: ClientId) -> bool {
        self.bots.iter().any(|bot| bot.id == id)
    }

    /// Distinct ids holding a slot: `taken` (humans and reserved connections, which may
    /// repeat and may already include joined bots) plus the bots of this roster.
    pub fn occupied(&self, taken: &[ClientId]) -> u32 {
        let ids: std::collections::HashSet<ClientId> = taken
            .iter()
            .copied()
            .chain(self.bots.iter().map(|bot| bot.id))
            .collect();
        ids.len() as u32
    }

    /// Bots that still fit next to everyone already holding a slot.
    pub fn room(&self, taken: &[ClientId]) -> u32 {
        MAX_PARTICIPANTS.saturating_sub(self.occupied(taken))
    }

    // `taken` are the ids real clients already own. A bot minted onto one of
    // them *is* that client as far as the roster is concerned: `is_bot` claims
    // the player, and the slot is dead weight because no system can drive an
    // id someone else is already playing.
    pub fn add_bots(&mut self, count: u32, taken: &[ClientId], dummy: bool) -> Vec<ClientId> {
        self.add_bots_with_identities(count, taken, dummy, &match_identities())
    }

    /// `add_bots` with an explicit identity order. Each new bot takes the first identity no
    /// present bot uses, so earlier bots keep their identity.
    pub(crate) fn add_bots_with_identities(
        &mut self,
        count: u32,
        taken: &[ClientId],
        dummy: bool,
        identities: &[BotIdentity],
    ) -> Vec<ClientId> {
        // Capped by the free slots at this moment, so requests made in one frame share
        // the same limit: each one sees the bots the previous one added.
        let count = count.min(self.room(taken));
        let seed = self.seed;
        let mut added = Vec::with_capacity(count as usize);
        for i in 0..count {
            let Some(id) = self.claim_id(taken) else {
                break;
            };
            let mut account = [0; 16];
            while account == [0; 16] {
                if let Err(error) = getrandom::fill(&mut account) {
                    diag::warn!(Sim, "bots: account identity creation failed: {error}");
                    return added;
                }
            }
            let brain = (!dummy)
                .then(|| HostController::new(seed ^ (u64::from(id.0) << 32) ^ u64::from(i)));
            let identity = identities
                .iter()
                .copied()
                .find(|identity| !self.bots.iter().any(|bot| bot.name == identity.name))
                .unwrap_or(BotIdentity::new("bot", 0, 0));
            self.bots.push(BotSlot {
                id,
                name: identity.name,
                rank: identity.rank,
                prestige: identity.prestige,
                account: sim::AccountId(account),
                brain,
                joined: false,
                class_picked_in: None,
                class_picks: 0,
            });
            added.push(id);
        }
        added
    }

    fn claim_id(&mut self, taken: &[ClientId]) -> Option<ClientId> {
        // Only `bots + taken` ids are spoken for, so one candidate more than
        // that always turns up a free one.
        let candidates = self
            .bots
            .len()
            .saturating_add(taken.len())
            .saturating_add(1);
        for _ in 0..candidates {
            let id = ClientId(self.next_client);
            self.next_client = self.next_client.wrapping_add(1).max(1);
            if !taken.contains(&id) && !self.is_bot(id) {
                return Some(id);
            }
        }
        None
    }
}

pub fn default_class_index(seed: u64, client: ClientId, pick: u32) -> u8 {
    let mix = seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(u64::from(client.0).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(u64::from(pick).wrapping_mul(0x94D0_49BB_1331_11EB));
    (mix >> 33) as u8
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn pool_is_ascii_unique_and_protocol_safe() {
        assert_eq!(BOT_NAMES.len(), 148);
        let mut visible = std::collections::HashSet::new();
        for &name in BOT_NAMES {
            assert!(name.is_ascii());
            let shown = display_name(name);
            assert!(shown.len() <= BOT_NAME_BYTES);
            assert!(visible.insert(shown), "duplicate display name: {shown}");
            let packed = entity_iw4::pack_client_state_name(shown);
            assert_eq!(&packed[..shown.len()], shown.as_bytes());
            assert_eq!(packed[shown.len()], 0);
        }
        assert!(BOT_NAMES.contains(&"GandalfTheGay"));
        assert_eq!(display_name("NoScopeNoBitches"), "NoScopeNoBitche");
    }

    #[test]
    fn shuffle_is_permutation_not_fixed_order() {
        let mut names = BOT_NAMES.to_vec();
        shuffle_names(&mut names, || u64::MAX);
        assert_ne!(names, BOT_NAMES);
        names.sort_unstable();
        let mut expected = BOT_NAMES.to_vec();
        expected.sort_unstable();
        assert_eq!(names, expected);
    }

    fn untagged(name: &str) -> &str {
        name.strip_prefix('[')
            .and_then(|rest| rest.split_once(']'))
            .map_or(name, |(_, base)| base)
    }

    #[test]
    fn tags_fit_the_protocol_and_never_change_a_name() {
        assert!(CLAN_TAGS.len() >= 60);
        let mut seen = std::collections::HashSet::new();
        for tag in CLAN_TAGS {
            assert!(tag.is_ascii() && !tag.contains(['[', ']']));
            assert!(seen.insert(*tag), "duplicate tag {tag}");
        }
        let originals: std::collections::HashSet<_> =
            BOT_NAMES.iter().map(|name| display_name(name)).collect();
        let drawn = draw_identities();
        assert_eq!(drawn.len(), BOT_NAMES.len());
        let mut bases = std::collections::HashSet::new();
        for identity in &drawn {
            let name = identity.name;
            assert!(name.is_ascii() && name.len() <= BOT_NAME_BYTES, "{name}");
            assert!(originals.contains(untagged(name)), "{name}");
            assert!(bases.insert(untagged(name)), "base name used twice: {name}");
            let packed = entity_iw4::pack_client_state_name(name);
            assert_eq!(&packed[..name.len()], name.as_bytes());
            assert!((0..=MAX_BOT_RANK).contains(&identity.rank));
            assert!((0..=MAX_BOT_PRESTIGE).contains(&identity.prestige));
        }
        assert_eq!(bases, originals);
    }

    #[test]
    fn tag_picker_prefers_irs_and_respects_gate_and_protocol_limit() {
        let mut irs_draw = [0, 9].into_iter();
        assert_eq!(with_tag("Yeet", &mut || irs_draw.next().unwrap()), "[IRS]Yeet");

        let mut skipped_draw = [9].into_iter();
        assert_eq!(with_tag("Yeet", &mut || skipped_draw.next().unwrap()), "Yeet");

        let mut too_long_draw = [0].into_iter();
        assert_eq!(
            with_tag("NoScopeNoBitche", &mut || too_long_draw.next().unwrap()),
            "NoScopeNoBitche"
        );
    }

    #[test]
    fn rank_and_prestige_draws_are_bounded() {
        // rank draw, then the prestige gate (6 and above: prestiged), then the emblem.
        let mut maximum_draw = [69, 9, 9].into_iter();
        assert_eq!(
            draw_rank_prestige(&mut || maximum_draw.next().unwrap()),
            (MAX_BOT_RANK, MAX_BOT_PRESTIGE)
        );

        let mut plain_draw = [0, 5].into_iter();
        assert_eq!(
            draw_rank_prestige(&mut || plain_draw.next().unwrap()),
            (0, 0)
        );

        let mut wrapped_draw = [70, 5].into_iter();
        assert_eq!(
            draw_rank_prestige(&mut || wrapped_draw.next().unwrap()),
            (0, 0)
        );
    }

    #[test]
    fn lobby_names_survive_count_changes_and_match_creation() {
        // One test body: the draw is shared state and tests run in parallel.
        let five = lobby_bot_names(5);
        assert_eq!(five, lobby_bot_names(5), "same count must stay stable");
        let six = lobby_bot_names(6);
        assert_eq!(six.len(), 6);
        assert_eq!(six, lobby_bot_names(6));
        assert_eq!(six[..5], five[..], "adding a bot must preserve existing names");
        let four = lobby_bot_names(4);
        assert_eq!(four[..], six[..4], "removing a bot must preserve existing names");

        // The match takes exactly the identities the lobby showed for this count.
        let shown = lobby_bot_names(MAX_HOST_BOTS);
        let mut roster = BotRoster::default();
        roster.add_bots(5, &[], true);
        roster.add_bots(MAX_HOST_BOTS - 5, &[], true);
        assert_eq!(
            roster.bots.iter().map(|b| b.name).collect::<Vec<_>>(),
            shown
        );
        assert_eq!(shown, lobby_bot_names(MAX_HOST_BOTS));

        // Only an explicit reset (a new lobby) draws a new order.
        reset_lobby_names();
        assert_ne!(lobby_bot_names(MAX_HOST_BOTS), shown);
    }

    #[test]
    fn a_full_lobby_has_unique_names_within_the_protocol_limit() {
        let identities = draw_identities();
        let shown: Vec<_> = identities.iter().take(MAX_PARTICIPANTS as usize).collect();
        let unique: std::collections::HashSet<_> = shown.iter().map(|identity| identity.name).collect();
        assert_eq!(unique.len(), shown.len());
        for identity in shown {
            let name = identity.name;
            assert!(name.is_ascii() && name.len() <= BOT_NAME_BYTES, "{name}");
            let packed = entity_iw4::pack_client_state_name(name);
            assert_eq!(&packed[..name.len()], name.as_bytes());
            assert_eq!(packed[name.len()], 0);
        }
    }
}
