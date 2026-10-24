mod controller;
mod intent;
mod memory;
mod motor;
mod nav;
mod observation;
mod plugin;
mod query;
mod roster;
#[cfg(test)]
mod roster_contract_tests;
mod sensor;
mod task;
pub mod unique_loadout;
mod weapon;

pub use controller::HostController;
pub use intent::{BotIntent, MotorReport, MoveMode, PathOutcome};
pub use nav::{
    NAV_HULL, NAV_SCHEMA, NavEdge, NavGraph, PathError, RouteStats, RouteStep, RouteWork,
    SupportProbe, TraversalKind, bake, bake_seeded, brush_bounds, find_path, find_route,
    find_route_resumable, playable_bounds, roam_node, supported_walk,
};
pub use observation::{
    BotEvent, BotObservation, Contact, DEFAULT_OBJECTIVE_RADIUS, KnowledgeSource, ModeObjective,
    ObjectiveAction, SelfState, Stance, TeamRole, Visibility, WeaponAction, WeaponClass,
    WeaponFacts, WeaponSlot,
};
pub use plugin::{BotWanted, BotsPlugin};
pub use query::{
    Budgeted, HullTrace, ObstacleKind, QueryCounters, QueryDenied, QueryResult, QuerySubsystem,
    SightSample, TraceBudget, WalkSample, WorldQuery,
};
pub use roster::{bot_room_for, lobby_bot_labels, lobby_bot_names, reset_lobby_names};
pub use roster::{
    BotAddQueue, BotAddRequest, BotFireQueue, BotHold, BotRoster, BotTpQueue, BotTpRequest,
    BotTpTarget, BotTpWhere, MAX_HOST_BOTS, MAX_PARTICIPANTS,
};
pub use sensor::{observe, observe_focused};
pub use task::{ActionStage, Decision, SwitchReason, Task, TaskKind};
pub use weapon::{WeaponAct, WeaponFailure, WeaponSkill, WeaponStatus};
