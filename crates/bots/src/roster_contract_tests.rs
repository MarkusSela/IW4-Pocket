//! Behavioral regression coverage for the 0.3.8 roster contract.
//! Compiled only by `cargo test -p bots`; exercises the real roster with an explicit
//! name order, so no test touches the shared lobby draw.

use crate::roster::{BotIdentity, BotRoster, MAX_HOST_BOTS, MAX_PARTICIPANTS};
use sim::ClientId;

const ORDER: [BotIdentity; 20] = [
    BotIdentity::new("n00", 0, 0), BotIdentity::new("n01", 1, 0),
    BotIdentity::new("n02", 2, 0), BotIdentity::new("n03", 3, 0),
    BotIdentity::new("n04", 4, 0), BotIdentity::new("n05", 5, 0),
    BotIdentity::new("n06", 6, 0), BotIdentity::new("n07", 7, 0),
    BotIdentity::new("n08", 8, 0), BotIdentity::new("n09", 9, 0),
    BotIdentity::new("n10", 10, 1), BotIdentity::new("n11", 11, 1),
    BotIdentity::new("n12", 12, 1), BotIdentity::new("n13", 13, 1),
    BotIdentity::new("n14", 14, 1), BotIdentity::new("n15", 15, 1),
    BotIdentity::new("n16", 16, 1), BotIdentity::new("n17", 17, 1),
    BotIdentity::new("n18", 18, 1), BotIdentity::new("n19", 19, 1),
];

fn add(roster: &mut BotRoster, count: u32, taken: &[ClientId]) -> usize {
    roster.add_bots_with_identities(count, taken, true, &ORDER).len()
}

/// What the host passes as `taken`: the local player plus the bots on the scoreboard.
fn taken_with_bots(roster: &BotRoster, humans: &[ClientId]) -> Vec<ClientId> {
    let mut taken = humans.to_vec();
    taken.extend(roster.bots.iter().map(|b| b.id));
    taken
}

#[test]
fn capacity_is_eighteen_with_the_player_included() {
    assert_eq!(MAX_PARTICIPANTS, 18);
    assert_eq!(MAX_HOST_BOTS, 17);
}

#[test]
fn one_human_fills_the_match_with_seventeen_bots() {
    let mut roster = BotRoster::default();
    let human = [ClientId(0)];
    assert_eq!(add(&mut roster, 17, &human), 17);
    assert_eq!(roster.occupied(&human), 18);
    assert_eq!(roster.room(&human), 0);
    assert!(!roster.bots.iter().any(|b| b.id == ClientId(0)));
}

#[test]
fn requests_beyond_capacity_are_clamped() {
    let mut roster = BotRoster::default();
    let human = [ClientId(0)];
    assert_eq!(add(&mut roster, u32::MAX, &human), 17);
    assert_eq!(roster.bots.len(), 17);
    assert_eq!(add(&mut roster, 1, &human), 0);
}

#[test]
fn two_humans_leave_room_for_sixteen_bots() {
    let mut roster = BotRoster::default();
    let humans = [ClientId(0), ClientId(77)];
    assert_eq!(add(&mut roster, 17, &humans), 16);
    assert_eq!(roster.occupied(&humans), 18);
    assert_eq!(add(&mut roster, 1, &humans), 0);
}

#[test]
fn scoreboard_and_reserved_ids_are_counted_once() {
    let mut roster = BotRoster::default();
    // The same human listed by the local client, the scoreboard and the connection.
    let humans = [ClientId(0), ClientId(0), ClientId(77), ClientId(77)];
    assert_eq!(roster.occupied(&humans), 2);
    assert_eq!(add(&mut roster, u32::MAX, &humans), 16);
    // Joined bots also show up on the scoreboard; they must not count twice.
    let taken = taken_with_bots(&roster, &humans);
    assert_eq!(roster.occupied(&taken), 18);
    assert_eq!(roster.room(&taken), 0);
    assert_eq!(add(&mut roster, 1, &taken), 0);
}

#[test]
fn occupied_ids_are_never_given_to_bots() {
    let mut roster = BotRoster::default();
    // The first ids the roster would mint are already held by humans.
    let humans = [ClientId(1), ClientId(2), ClientId(3)];
    assert_eq!(add(&mut roster, u32::MAX, &humans), 15);
    let ids: std::collections::HashSet<_> = roster.bots.iter().map(|b| b.id).collect();
    assert_eq!(ids.len(), 15, "bot ids must be distinct");
    assert!(humans.iter().all(|id| !ids.contains(id)));
}

#[test]
fn several_requests_in_one_frame_share_the_limit() {
    let mut roster = BotRoster::default();
    let human = [ClientId(0)];
    // Same `taken` snapshot for every request, as in one frame.
    assert_eq!(add(&mut roster, 9, &human), 9);
    assert_eq!(add(&mut roster, 9, &human), 8);
    assert_eq!(add(&mut roster, u32::MAX, &human), 0);
    assert_eq!(roster.bots.len(), 17);
}

#[test]
fn a_joining_human_takes_the_slot_a_bot_would_have_used() {
    let mut roster = BotRoster::default();
    assert_eq!(add(&mut roster, 17, &[ClientId(0)]), 17);
    // A second human connects: no room is left for another bot...
    let humans = [ClientId(0), ClientId(900)];
    assert_eq!(roster.room(&humans), 0);
    assert_eq!(add(&mut roster, 1, &humans), 0);
    // ...until enough bots are removed: 2 humans + 17 bots is 19, so one pop
    // only brings the lobby back to 18 and the second human keeps that seat.
    roster.bots.pop();
    assert_eq!(roster.room(&humans), 0);
    assert_eq!(add(&mut roster, 1, &humans), 0);
    roster.bots.pop();
    assert_eq!(roster.room(&humans), 1);
    assert_eq!(add(&mut roster, 1, &humans), 1);
}

#[test]
fn names_stay_with_their_bots_across_removal_and_addition() {
    let mut roster = BotRoster::default();
    let human = [ClientId(0)];
    add(&mut roster, 5, &human);
    let identities: Vec<_> = roster
        .bots
        .iter()
        .map(|b| BotIdentity::new(b.name, b.rank, b.prestige))
        .collect();
    assert_eq!(identities, ORDER[..5]);

    // Adding never renames: the newcomer takes the next name in the order.
    add(&mut roster, 1, &human);
    assert_eq!(
        roster.bots[..5]
            .iter()
            .map(|b| BotIdentity::new(b.name, b.rank, b.prestige))
            .collect::<Vec<_>>(),
        identities
    );
    assert_eq!(
        BotIdentity::new(
            roster.bots[5].name,
            roster.bots[5].rank,
            roster.bots[5].prestige,
        ),
        ORDER[5]
    );

    // Removing from the middle keeps everyone else's name, and the freed name is
    // handed out again before any later one.
    let removed = roster.bots.remove(2);
    assert_eq!(BotIdentity::new(removed.name, removed.rank, removed.prestige), ORDER[2]);
    let survivors: Vec<_> = roster
        .bots
        .iter()
        .map(|b| (b.id, BotIdentity::new(b.name, b.rank, b.prestige)))
        .collect();
    add(&mut roster, 1, &human);
    for (id, identity) in survivors {
        let bot = roster.bots.iter().find(|b| b.id == id).unwrap();
        assert_eq!(BotIdentity::new(bot.name, bot.rank, bot.prestige), identity);
    }
    let replacement = roster.bots.last().unwrap();
    assert_eq!(
        BotIdentity::new(replacement.name, replacement.rank, replacement.prestige),
        ORDER[2]
    );
}

#[test]
fn bot_names_are_unique_in_a_full_match() {
    let mut roster = BotRoster::default();
    add(&mut roster, u32::MAX, &[ClientId(0)]);
    let unique: std::collections::HashSet<_> = roster.bots.iter().map(|b| b.name).collect();
    assert_eq!(unique.len(), roster.bots.len());
    assert!(roster.bots.iter().all(|b| b.name != "bot"));
}

#[test]
fn the_relay_is_told_every_id_the_bots_hold() {
    // A bot that is minted but not on the scoreboard yet still holds its slot; the plugin
    // publishes these ids so the relay never gives one of them to a human.
    let mut roster = BotRoster::default();
    let human = [ClientId(0)];
    assert_eq!(add(&mut roster, 5, &human), 5);
    let mut held: Vec<u32> = crate::plugin::reserved_ids_of(&roster);
    held.sort_unstable();
    let mut expected: Vec<u32> = roster.bots.iter().map(|b| b.id.0).collect();
    expected.sort_unstable();
    assert_eq!(held, expected);
    assert_eq!(held.len(), 5);
    assert!(!held.contains(&0), "the local player is not a bot");
}
