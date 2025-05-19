use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Default)]
struct Turn(u32);

#[derive(Event)]
struct TurnDeltaEvent(u32);

#[derive(Resource, Default)]
struct Notificiations(Vec<Notification>);

struct Notification {
    message: String,
    is_unread: bool,
}

#[derive(Event)]
struct NotificationEvent(String);

#[derive(Component)]
struct Hero;

#[derive(Component)]
struct Person {
    personality: Personality,
    relationships: HashMap<Entity, i32>,
}

#[derive(Debug)]
enum Personality {
    Friendly,       // +1 opinion of party members after questing together, regardless of outcome
    ResultOriented, // +1 opinion of party members if successful, -1 if not
    Mirror,         // Moves toward the other person's opinion of them
    Judgmental,     // -2 opinion of party members if they get injured, +1 otherwise.
    Learner,        // +1 opinion of anyone stronger, -1 of anyone weaker
    Teacher,        // +1 opinion of anyone weaker, -1 of anyone stronger
}

#[derive(Component, Debug)]
enum HeroClass {
    Warrior,
    Tank,
    Support,
}

#[derive(Component)]
struct LevelState {
    level: u32,
    exp: u32,
    exp_to_next: u32,
}

struct Item {
    class: HeroClass,
}

#[derive(Component)]
struct Quest;

#[derive(Component)]
struct QuestDescription {
    difficulty_level: u32,
    turns_to_complete: u32,
    exp_reward: u32,
    gold_reward: u32,
    item_reward: Option<Item>,
    turns_to_expiry: u32,
}

#[derive(Component)]
struct TurnTimer {
    initial_value: u32, // Number of turns this timer will take (or has taken) to complete.
    turns_remaining: u32, // Starts equal to initial_value and counts down to 0.
}

#[derive(Event)]
struct TurnTimerComplete(Entity);

#[derive(Bundle)]
struct HeroBundle {
    marker: Hero,
    level: LevelState,
    class: HeroClass,
    person: Person,
}

#[derive(Bundle)]
struct QuestBundle {
    marker: Quest,
    description: QuestDescription,
    progress: TurnTimer,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Turn>()
        .init_resource::<Notificiations>()
        .add_event::<NotificationEvent>()
        .add_event::<TurnDeltaEvent>()
        .add_event::<TurnTimerComplete>()
        .add_systems(Startup, setup)
        .add_systems(Update, log_new_hero)
        .add_systems(Update, handle_notifcation_events)
        .add_systems(Update, advance_turn)
        .add_systems(Update, advance_turn_timer)
        .run();
}

fn setup(mut commands: Commands) {
    // Setup some initial heros and quests
    commands.spawn(HeroBundle {
        marker: Hero,
        level: LevelState {
            level: 1,
            exp: 0,
            exp_to_next: 100,
        },
        class: HeroClass::Warrior,
        person: Person {
            personality: Personality::Friendly,
            relationships: HashMap::new(),
        },
    });
    commands.spawn(HeroBundle {
        marker: Hero,
        level: LevelState {
            level: 1,
            exp: 0,
            exp_to_next: 100,
        },
        class: HeroClass::Tank,
        person: Person {
            personality: Personality::ResultOriented,
            relationships: HashMap::new(),
        },
    });

    commands.spawn(QuestBundle {
        marker: Quest,
        description: QuestDescription {
            difficulty_level: 1,
            turns_to_complete: 5,
            exp_reward: 50,
            gold_reward: 100,
            item_reward: None,
            turns_to_expiry: 10,
        },
        progress: TurnTimer {
            initial_value: 5,
            turns_remaining: 5,
        },
    });
}

fn log_new_hero(
    query: Query<(&LevelState, &HeroClass, &Person), Added<Hero>>,
    mut ev_notify: EventWriter<NotificationEvent>,
) {
    for (level, class, person) in query.iter() {
        ev_notify.write(NotificationEvent(format!(
            "New hero created: Level: {}, Class: {:?}, Personality: {:?}",
            level.level, class, person.personality
        )));
    }
}

fn handle_notifcation_events(
    mut ev_notifcations: EventReader<NotificationEvent>,
    mut notifications: ResMut<Notificiations>,
) {
    for event in ev_notifcations.read() {
        let n = Notification {
            message: event.0.clone(),
            is_unread: true,
        };
        info!("Notification: {}", n.message);
        notifications.0.push(n);
    }
}

// When TurnDelta event happens, advance Turn resource
fn advance_turn(
    mut turn: ResMut<Turn>,
    mut ev_turn_delta: EventReader<TurnDeltaEvent>,
    mut ev_notify: EventWriter<NotificationEvent>
) {
    let total_delta: u32 = ev_turn_delta.read().map(|e| e.0).sum();
    turn.0 += total_delta;
    ev_notify.write(NotificationEvent(format!(
        "Turn advanced by {}. Current turn: {}",
        total_delta, turn.0
    )));
}

// On TurnDelta event, for TurnTimer components, advance progress. If progress complete, emit TurnTimerComplete event.
fn advance_turn_timer(
    mut ev_turn_delta: EventReader<TurnDeltaEvent>,
    mut query: Query<(Entity, &mut TurnTimer)>,
    mut ev_turn_timer_complete: EventWriter<TurnTimerComplete>,
) {
    let turn_delta: u32 = ev_turn_delta.read().map(|e| e.0).sum();
    query.iter_mut().for_each(|(entity, mut timer)| {
        timer.turns_remaining = timer.turns_remaining.saturating_sub(turn_delta);
        if timer.turns_remaining == 0 {
            ev_turn_timer_complete.write(TurnTimerComplete(entity));
            info!("Turn timer complete for entity: {:?}", entity);
        }
    });
}

// On TurnTimerComplete event, for entities with Quest and OnQuestBoard markers, despawn the entity.

// On TurnTimerComplete event, for entities with Quest and QuestInProgress markers, a ProbabilityOfSuccess.
