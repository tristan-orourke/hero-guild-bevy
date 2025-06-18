use bevy::prelude::*;
use std::collections::HashMap;
use std::ops::{Add, Sub};
use rand::{distributions::Distribution, distributions::Bernoulli};

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

// Quest status markers
#[derive(Component)]
struct QuestStatusAvailable;
#[derive(Component)]
struct QuestStatusInProgress;

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
struct TurnTimerCompleteEvent(Entity); // An event indiciating a TurnTimer attached to an entity has completed.

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
    status: QuestStatusAvailable,
}

#[derive(Event)]
struct StartQuestEvent {
    quest: Entity,
    heroes: Vec<Entity>
}

#[derive(Component)]
struct ProbabilityOfSuccess(Percent);

#[derive(Debug, PartialEq, Copy, Clone)]
struct Percent(i32); // Represents a percentage value, normally 0-100, but we allow for negative or >100 values while adding values together.
impl Add for Percent {
    type Output = Percent;

    fn add(self, rhs: Percent) -> Percent {
        Percent(self.0 + rhs.0)
    }
}

impl Sub for Percent {
    type Output = Percent;

    fn sub(self, rhs: Percent) -> Percent {
        Percent(self.0 - rhs.0)
    }
}

impl Percent {
    fn distribution(&self) -> Distribution<bool> {
        Bernoulli::from_ratio(self.0.clamp(0, 100), 100)
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Turn>()
        .init_resource::<Notificiations>()
        .add_event::<NotificationEvent>()
        .add_event::<TurnDeltaEvent>()
        .add_event::<TurnTimerCompleteEvent>()
        .add_systems(Startup, setup)
        .add_systems(Update, log_new_hero)
        .add_systems(Update, handle_notifcation_events)
        .add_systems(Update, advance_turn)
        .add_systems(Update, advance_turn_timer)
        .add_systems(Update, expire_quest)
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
        status: QuestStatusAvailable,
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
    mut ev_notify: EventWriter<NotificationEvent>,
) {
    let total_delta: u32 = ev_turn_delta.read().map(|e| e.0).sum();
    turn.0 += total_delta;
    ev_notify.write(NotificationEvent(format!(
        "Turn advanced by {}. Current turn: {}",
        total_delta, turn.0
    )));
}


#[test]
fn turn_delta_did_advance_turn() {
    let mut app = App::new();
    app.init_resource::<Turn>();
    app.add_event::<TurnDeltaEvent>();
    app.add_event::<NotificationEvent>();

    // Insert a NotificationEvent collector
    app.init_resource::<Notificiations>();

    // Add the system under test
    app.add_systems(Update, advance_turn);

    // Send multiple TurnDeltaEvents
    app.world_mut().resource_mut::<Events<TurnDeltaEvent>>().send(TurnDeltaEvent(3));
    app.world_mut().resource_mut::<Events<TurnDeltaEvent>>().send(TurnDeltaEvent(2));

    // Run the system
    app.update();

    // Check that the turn was incremented
    assert_eq!(app.world().resource::<Turn>().0, 5);
}

#[test]
fn turn_delta_did_send_notification() {
    let mut app = App::new();
    app.init_resource::<Turn>();
    app.add_event::<TurnDeltaEvent>();
    app.add_event::<NotificationEvent>();

    // Insert a NotificationEvent collector
    app.init_resource::<Notificiations>();

    // Add the system under test
    app.add_systems(Update, advance_turn);

    // Send a TurnDeltaEvent
    app.world_mut().resource_mut::<Events<TurnDeltaEvent>>().send(TurnDeltaEvent(1));

    // Run the system
    app.update();

    // Check that a notification was sent
    let notification_events = app.world().resource::<Events<NotificationEvent>>();
    let mut notification_reader = notification_events.get_cursor();
    let notification = notification_reader.read(notification_events).next().unwrap();
    assert_eq!(notification.0, "Turn advanced by 1. Current turn: 1");
    
}

// On TurnDelta event, for TurnTimer components, advance progress. If progress complete, emit TurnTimerComplete event.
fn advance_turn_timer(
    mut ev_turn_delta: EventReader<TurnDeltaEvent>,
    mut query: Query<(Entity, &mut TurnTimer)>,
    mut ev_turn_timer_complete: EventWriter<TurnTimerCompleteEvent>,
) {
    let turn_delta: u32 = ev_turn_delta.read().map(|e| e.0).sum();
    query
        .iter_mut()
        .filter(|(_, timer)| timer.turns_remaining > 0) // Only process timers that aren't yet complete
        .for_each(|(entity, mut timer)| {
            timer.turns_remaining = timer.turns_remaining.saturating_sub(turn_delta);
            if timer.turns_remaining == 0 {
                ev_turn_timer_complete.write(TurnTimerCompleteEvent(entity));
                info!("Turn timer complete for entity: {:?}", entity);
            }
        });
}

#[test]
fn turn_delta_did_advance_turn_timer() {
    let mut app = App::new();
    app.add_event::<TurnDeltaEvent>();
    app.add_event::<TurnTimerCompleteEvent>();

    // Add a TurnTimer component to an entity
    let entity = app.world_mut().spawn((
        TurnTimer {
            initial_value: 5,
            turns_remaining: 5,
        },
    )).id();

    // Add the system under test
    app.add_systems(Update, advance_turn_timer);

    // Send a TurnDeltaEvent
    app.world_mut().resource_mut::<Events<TurnDeltaEvent>>().send(TurnDeltaEvent(1));
    app.world_mut().resource_mut::<Events<TurnDeltaEvent>>().send(TurnDeltaEvent(2));

    // Run the system
    app.update();

    // Check that the timer was decremented
    let timer = app.world().get::<TurnTimer>(entity).unwrap();
    assert_eq!(timer.turns_remaining, 2);
}

#[test]
fn advance_turn_timer_did_emit_completion_event() {
    let mut app = App::new();
    app.add_event::<TurnDeltaEvent>();
    app.add_event::<TurnTimerCompleteEvent>();

    // Add a TurnTimer component to an entity
    let entity = app.world_mut().spawn((
        TurnTimer {
            initial_value: 5,
            turns_remaining: 5,
        },
    )).id();

    // Add the system under test
    app.add_systems(Update, advance_turn_timer);

    // Send a TurnDeltaEvent that will complete the timer
    app.world_mut().resource_mut::<Events<TurnDeltaEvent>>().send(TurnDeltaEvent(5));

    // Run the system
    app.update();

    // Check that the TurnTimerCompleteEvent was emitted
    let turn_timer_events = app.world().resource::<Events<TurnTimerCompleteEvent>>();
    let mut reader = turn_timer_events.get_cursor();
    let event = reader.read(turn_timer_events).next().unwrap();
    assert_eq!(event.0, entity);
}

#[test]
fn advance_turn_timer_doesnt_go_past_zero() {
    let mut app = App::new();
    app.add_event::<TurnDeltaEvent>();
    app.add_event::<TurnTimerCompleteEvent>();

    // Add a TurnTimer component to an entity
    let entity = app.world_mut().spawn((
        TurnTimer {
            initial_value: 5,
            turns_remaining: 5,
        },
    )).id();

    // Add the system under test
    app.add_systems(Update, advance_turn_timer);

    // Send a TurnDeltaEvent that will go past zero
    app.world_mut().resource_mut::<Events<TurnDeltaEvent>>().send(TurnDeltaEvent(5));

    // Run the system
    app.update();

    // Check that the timer was not decremented below zero
    let timer = app.world().get::<TurnTimer>(entity).unwrap();
    assert_eq!(timer.turns_remaining, 0);
}

// When turn timer completes for Available quest, despawn the quest and notify.
fn expire_quest(
    mut commands: Commands,
    mut ev_turn_timer_complete: EventReader<TurnTimerCompleteEvent>,
    query: Query<
        Entity,
        (
            With<Quest>,
            With<QuestStatusAvailable>,
        ),
    >,
    mut ev_notify: EventWriter<NotificationEvent>,
) {
    for TurnTimerCompleteEvent(entity) in ev_turn_timer_complete.read() {
        if let Ok(_) = query.get(*entity) {
            commands.entity(*entity).despawn();
            ev_notify.write(NotificationEvent(format!(
                "An available quest expired: entity {:?}",
                *entity
            )));
        }
    }
}

#[test]
fn expire_quest_despawns_available_quest() {
    let mut app = App::new();
    app.add_event::<TurnTimerCompleteEvent>();
    app.add_event::<NotificationEvent>();

    // Add an available quest with a TurnTimer
    let entity = app.world_mut().spawn((
        Quest,
        QuestStatusAvailable,
        TurnTimer {
            initial_value: 5,
            turns_remaining: 0,
        },
    )).id();

    // Add the system under test
    app.add_systems(Update, expire_quest);

    // Send a TurnTimerCompleteEvent for the quest
    app.world_mut().resource_mut::<Events<TurnTimerCompleteEvent>>()
        .send(TurnTimerCompleteEvent(entity));

    // Run the system
    app.update();

    // Check that the quest was despawned
    assert!(!app.world().get::<Quest>(entity).is_some());

    // Check that a notification was sent
    let notification_events = app.world().resource::<Events<NotificationEvent>>();
    let mut reader = notification_events.get_cursor();
    let notification = reader.read(notification_events).next().unwrap();
    assert_eq!(notification.0, format!("An available quest expired: entity {:?}", entity));
}

// When timer completes for an InProgress quest with a percentage of success, determine result, despawn the quest, and emit a End of Quest event.
// TODO

// When a quest is started, set quest and hero statuses, and begin the quest timer.
fn start_quest(
    mut commands: Commands,
    mut ev_start_quest: EventReader<StartQuestEvent>,
    quests_query: Query<(&QuestDescription), With<Quest>>,
    heroes_query: Query<(&LevelState, &Person), With<Hero>>
) {
    for StartQuestEvent { quest, heroes } in ev_start_quest.read() {
        if let Ok(description) = quests_query.get(*quest) {
            // Set the quest status to InProgress
            commands.entity(*quest)
                .remove::<QuestStatusAvailable>()
                .insert(QuestStatusInProgress)
                // TODO: Will this work if quest already has a TurnTimer?
                .insert(TurnTimer {
                    initial_value: description.turns_to_complete,
                    turns_remaining: description.turns_to_complete,
                });

            // Assign heros to quest, using ChildOf/Children relationships
            for hero in heroes.iter() {
                commands.entity(*hero).insert(ChildOf(*quest));
            }

            // Determine probability of success based on hero levels and quest difficulty
            // TODO:
        }
    }
}

fn probability_of_quest_success(difficulty_level: u32, heros: &Vec<(LevelState, Person)>) -> Percent {
    let total_effectiveness: i32 = heros.iter().map(|(level, _)| -> i32 {
        let baseline_effectiveness = 70; // Effectiveness percentage if hero level matches difficulty level
        let diff_per_level = 20; // Effectiveness increases by 20% for each level above difficulty level
        let level_diff = level.level as i32 - difficulty_level as i32; // Positive if hero is stronger than difficulty level
        baseline_effectiveness + (level_diff * diff_per_level)
    }).sum();
    let average_effectiveness = total_effectiveness / heros.len() as i32;
    Percent(average_effectiveness)
}

#[test]
fn probability_of_quest_success_finds_expected_values() {
    // TODO: derive default to make it easier to create test data
    let heros_lvl_3 = vec![
        (LevelState { level: 3, exp: 0, exp_to_next: 100 }, Person { personality: Personality::Friendly, relationships: HashMap::new() }),
        (LevelState { level: 3, exp: 0, exp_to_next: 100 }, Person { personality: Personality::ResultOriented, relationships: HashMap::new() }),
        (LevelState { level: 3, exp: 0, exp_to_next: 100 }, Person { personality: Personality::Learner, relationships: HashMap::new() }),
    ];
    assert_eq!(probability_of_quest_success(5, &heros_lvl_3), Percent(30));
    assert_eq!(probability_of_quest_success(4, &heros_lvl_3), Percent(50));
    assert_eq!(probability_of_quest_success(3, &heros_lvl_3), Percent(70));
    assert_eq!(probability_of_quest_success(2, &heros_lvl_3), Percent(90));
    assert_eq!(probability_of_quest_success(1, &heros_lvl_3), Percent(110));

    let heros_avg_3 = vec![
        (LevelState { level: 3, exp: 0, exp_to_next: 100 }, Person { personality: Personality::Friendly, relationships: HashMap::new() }),
        (LevelState { level: 2, exp: 0, exp_to_next: 100 }, Person { personality: Personality::ResultOriented, relationships: HashMap::new() }),
        (LevelState { level: 4, exp: 0, exp_to_next: 100 }, Person { personality: Personality::Learner, relationships: HashMap::new() }),
    ];
    assert_eq!(probability_of_quest_success(5, &heros_avg_3), Percent(30));
    assert_eq!(probability_of_quest_success(4, &heros_avg_3), Percent(50));
    assert_eq!(probability_of_quest_success(3, &heros_avg_3), Percent(70));
    assert_eq!(probability_of_quest_success(2, &heros_avg_3), Percent(90));
    assert_eq!(probability_of_quest_success(1, &heros_avg_3), Percent(110));

    let heros_avg_fractional = vec![
        (LevelState { level: 3, exp: 0, exp_to_next: 100 }, Person { personality: Personality::Friendly, relationships: HashMap::new() }),
        (LevelState { level: 2, exp: 0, exp_to_next: 100 }, Person { personality: Personality::ResultOriented, relationships: HashMap::new() }),
        (LevelState { level: 5, exp: 0, exp_to_next: 100 }, Person { personality: Personality::Learner, relationships: HashMap::new() }),
    ];
    assert_eq!(probability_of_quest_success(4, &heros_avg_fractional), Percent(93));
    assert_eq!(probability_of_quest_success(3, &heros_avg_fractional), Percent(73));
    assert_eq!(probability_of_quest_success(3, &heros_avg_fractional), Percent(53));
}