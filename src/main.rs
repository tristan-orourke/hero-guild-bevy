use bevy::prelude::*;
use rand::{
    SeedableRng,
    distr::{Bernoulli, Distribution},
};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use std::ops::{Add, Sub};

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

#[derive(Resource)]
struct RandomSource(ChaCha8Rng);

#[derive(Resource, Default)]
struct Guild {
    gold: u32,
}

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

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Component, Clone, Copy, Debug)]
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
    heroes: Vec<Entity>,
}

#[derive(Event)]
struct QuestCompleteEvent {
    quest_description: QuestDescription,
    heroes: Vec<Entity>,          // Heroes that completed the quest
    success_probability: Percent, // Probability of success for the quest
    is_successful: bool,          // Whether the quest was successful or not
    exp_reward: u32,              // Experience reward for the heroes
    gold_reward: u32,             // Gold reward for the guild
                                  // TODO: implement items
                                  // TODO: implement relationship updates
                                  // TODO: implement injuries
}

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
    fn distribution(&self) -> Bernoulli {
        Bernoulli::from_ratio(self.0.clamp(0, 100) as u32, 100).unwrap()
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Turn>()
        .init_resource::<Notificiations>()
        .init_resource::<Guild>()
        .add_event::<NotificationEvent>()
        .add_event::<TurnDeltaEvent>()
        .add_event::<TurnTimerCompleteEvent>()
        .add_systems(Startup, setup)
        .add_systems(Update, log_new_hero)
        .add_systems(Update, handle_notifcation_events)
        .add_systems(Update, advance_turn)
        .add_systems(Update, advance_turn_timer)
        .add_systems(Update, expire_quest)
        .add_systems(Update, start_quest)
        .add_systems(Update, complete_quest)
        .add_systems(Update, complete_quest_assign_exp)
        .add_systems(Update, complete_quest_updates_guild)
        .add_systems(Update, complete_quest_send_notification)
        .run();
}

fn setup(mut commands: Commands) {
    let seeded_rng = ChaCha8Rng::seed_from_u64(42);
    commands.insert_resource(RandomSource(seeded_rng));

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
    app.world_mut()
        .resource_mut::<Events<TurnDeltaEvent>>()
        .send(TurnDeltaEvent(3));
    app.world_mut()
        .resource_mut::<Events<TurnDeltaEvent>>()
        .send(TurnDeltaEvent(2));

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
    app.world_mut()
        .resource_mut::<Events<TurnDeltaEvent>>()
        .send(TurnDeltaEvent(1));

    // Run the system
    app.update();

    // Check that a notification was sent
    let notification_events = app.world().resource::<Events<NotificationEvent>>();
    let mut notification_reader = notification_events.get_cursor();
    let notification = notification_reader
        .read(notification_events)
        .next()
        .unwrap();
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
    let entity = app
        .world_mut()
        .spawn((TurnTimer {
            initial_value: 5,
            turns_remaining: 5,
        },))
        .id();

    // Add the system under test
    app.add_systems(Update, advance_turn_timer);

    // Send a TurnDeltaEvent
    app.world_mut()
        .resource_mut::<Events<TurnDeltaEvent>>()
        .send(TurnDeltaEvent(1));
    app.world_mut()
        .resource_mut::<Events<TurnDeltaEvent>>()
        .send(TurnDeltaEvent(2));

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
    let entity = app
        .world_mut()
        .spawn((TurnTimer {
            initial_value: 5,
            turns_remaining: 5,
        },))
        .id();

    // Add the system under test
    app.add_systems(Update, advance_turn_timer);

    // Send a TurnDeltaEvent that will complete the timer
    app.world_mut()
        .resource_mut::<Events<TurnDeltaEvent>>()
        .send(TurnDeltaEvent(5));

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
    let entity = app
        .world_mut()
        .spawn((TurnTimer {
            initial_value: 5,
            turns_remaining: 5,
        },))
        .id();

    // Add the system under test
    app.add_systems(Update, advance_turn_timer);

    // Send a TurnDeltaEvent that will go past zero
    app.world_mut()
        .resource_mut::<Events<TurnDeltaEvent>>()
        .send(TurnDeltaEvent(5));

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
    query: Query<Entity, (With<Quest>, With<QuestStatusAvailable>)>,
    mut ev_notify: EventWriter<NotificationEvent>,
) {
    for TurnTimerCompleteEvent(entity) in ev_turn_timer_complete.read() {
        if query.get(*entity).is_ok() {
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
    let entity = app
        .world_mut()
        .spawn((
            Quest,
            QuestStatusAvailable,
            TurnTimer {
                initial_value: 5,
                turns_remaining: 0,
            },
        ))
        .id();

    // Add the system under test
    app.add_systems(Update, expire_quest);

    // Send a TurnTimerCompleteEvent for the quest
    app.world_mut()
        .resource_mut::<Events<TurnTimerCompleteEvent>>()
        .send(TurnTimerCompleteEvent(entity));

    // Run the system
    app.update();

    // Check that the quest was despawned
    assert!(!app.world().get::<Quest>(entity).is_some());

    // Check that a notification was sent
    let notification_events = app.world().resource::<Events<NotificationEvent>>();
    let mut reader = notification_events.get_cursor();
    let notification = reader.read(notification_events).next().unwrap();
    assert_eq!(
        notification.0,
        format!("An available quest expired: entity {:?}", entity)
    );
}

// When timer completes for an InProgress quest with a percentage of success, determine result, despawn the quest, and emit a End of Quest event.
// TODO

// When a quest is started, set quest and hero statuses, and begin the quest timer.
fn start_quest(
    mut commands: Commands,
    mut ev_start_quest: EventReader<StartQuestEvent>,
    quests_query: Query<&QuestDescription, With<Quest>>,
) {
    for StartQuestEvent { quest, heroes } in ev_start_quest.read() {
        if let Ok(description) = quests_query.get(*quest) {
            // Set the quest status to InProgress
            commands
                .entity(*quest)
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
        }
    }
}

// When a in-progress quest is complete, determine success and other outcomes, despawn the quest, and create a QuestCompleteEvent.
fn complete_quest(
    mut commands: Commands,
    mut ev_turn_timer_complete: EventReader<TurnTimerCompleteEvent>,
    mut random_src: ResMut<RandomSource>,
    quests_query: Query<(&QuestDescription, &Children), (With<Quest>, With<QuestStatusInProgress>)>,
    heroes_query: Query<(&LevelState, &Person), With<Hero>>,
    mut ev_quest_complete: EventWriter<QuestCompleteEvent>,
) {
    for TurnTimerCompleteEvent(entity) in ev_turn_timer_complete.read() {
        if let Ok((description, children)) = quests_query.get(*entity) {
            let heroes: Vec<_> = children
                .iter()
                .map(|child| heroes_query.get(child).unwrap())
                .collect();
            let success_probability =
                probability_of_quest_success(description.difficulty_level, &heroes[..]);
            let rng = &mut random_src.0;
            let is_successful = success_probability.distribution().sample(rng);
            ev_quest_complete.write(QuestCompleteEvent {
                quest_description: *description,
                heroes: children.to_vec(), // Heroes that were part of the quest
                success_probability,
                is_successful,
                exp_reward: description.exp_reward, // Heroes gain experience regardless of success
                gold_reward: if is_successful {
                    description.gold_reward
                } else {
                    0
                }, // Guild gains gold only on success,
            });
            // Remove ChildOf components before despawning quest, or heroes will be despawned with it.
            for child in children.iter() {
                commands.entity(child).remove::<ChildOf>();
            }
            commands.entity(*entity).despawn(); // Despawn the quest entity
        }
    }
}

#[test]
fn complete_quest_despawns_quest_and_unlinks_heroes() {
    let mut app = App::new();
    app.add_event::<TurnTimerCompleteEvent>();
    app.add_event::<QuestCompleteEvent>();
    app.insert_resource::<RandomSource>(RandomSource(ChaCha8Rng::seed_from_u64(42)));

    // Add a quest with a TurnTimer
    let quest_entity = app
        .world_mut()
        .spawn((
            Quest,
            QuestStatusInProgress,
            TurnTimer {
                initial_value: 5,
                turns_remaining: 0,
            },
            QuestDescription {
                difficulty_level: 1,
                turns_to_complete: 5,
                exp_reward: 50,
                gold_reward: 100,
                item_reward: None,
                turns_to_expiry: 10,
            },
        ))
        .id();

    // Add a hero to the quest
    let hero_entity = app
        .world_mut()
        .spawn(HeroBundle {
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
        })
        .id();

    // Link hero to quest
    app.world_mut()
        .entity_mut(hero_entity)
        .insert(ChildOf(quest_entity));

    // Add the system under test
    app.add_systems(Update, complete_quest);

    // Send a TurnTimerCompleteEvent for the quest
    app.world_mut()
        .resource_mut::<Events<TurnTimerCompleteEvent>>()
        .send(TurnTimerCompleteEvent(quest_entity));

    // Run the system
    app.update();

    // Check that the quest was despawned
    assert!(!app.world().get::<Quest>(quest_entity).is_some());

    // Check that the hero is still present and not despawned
    assert!(app.world().get::<Hero>(hero_entity).is_some());

    // Check that the ChildOf component was removed from the hero
    assert!(!app.world().get::<ChildOf>(hero_entity).is_some());

    // Check that a QuestCompleteEvent was emitted, and contains reference to hero
    let quest_complete_events = app.world().resource::<Events<QuestCompleteEvent>>();
    let mut reader = quest_complete_events.get_cursor();
    let event = reader.read(quest_complete_events).next().unwrap();
    assert_eq!(event.heroes, vec![hero_entity]);
    assert_eq!(event.quest_description.difficulty_level, 1);
}

fn probability_of_quest_success(
    difficulty_level: u32,
    heros: &[(&LevelState, &Person)],
) -> Percent {
    let total_effectiveness: i32 = heros
        .iter()
        .map(|(level, _)| -> i32 {
            let baseline_effectiveness = 70; // Effectiveness percentage if hero level matches difficulty level
            let diff_per_level = 20; // Effectiveness increases by 20% for each level above difficulty level
            let level_diff = level.level as i32 - difficulty_level as i32; // Positive if hero is stronger than difficulty level
            baseline_effectiveness + (level_diff * diff_per_level)
        })
        .sum();
    let average_effectiveness = total_effectiveness / heros.len() as i32;
    Percent(average_effectiveness)
}

#[test]
fn probability_of_quest_success_finds_expected_values() {
    // TODO: derive default to make it easier to create test data
    let heros_lvl_3 = [
        (
            &LevelState {
                level: 3,
                exp: 0,
                exp_to_next: 100,
            },
            &Person {
                personality: Personality::Friendly,
                relationships: HashMap::new(),
            },
        ),
        (
            &LevelState {
                level: 3,
                exp: 0,
                exp_to_next: 100,
            },
            &Person {
                personality: Personality::ResultOriented,
                relationships: HashMap::new(),
            },
        ),
        (
            &LevelState {
                level: 3,
                exp: 0,
                exp_to_next: 100,
            },
            &Person {
                personality: Personality::Learner,
                relationships: HashMap::new(),
            },
        ),
    ];
    assert_eq!(probability_of_quest_success(5, &heros_lvl_3), Percent(30));
    assert_eq!(probability_of_quest_success(4, &heros_lvl_3), Percent(50));
    assert_eq!(probability_of_quest_success(3, &heros_lvl_3), Percent(70));
    assert_eq!(probability_of_quest_success(2, &heros_lvl_3), Percent(90));
    assert_eq!(probability_of_quest_success(1, &heros_lvl_3), Percent(110));

    let heros_avg_3 = [
        (
            &LevelState {
                level: 3,
                exp: 0,
                exp_to_next: 100,
            },
            &Person {
                personality: Personality::Friendly,
                relationships: HashMap::new(),
            },
        ),
        (
            &LevelState {
                level: 2,
                exp: 0,
                exp_to_next: 100,
            },
            &Person {
                personality: Personality::ResultOriented,
                relationships: HashMap::new(),
            },
        ),
        (
            &LevelState {
                level: 4,
                exp: 0,
                exp_to_next: 100,
            },
            &Person {
                personality: Personality::Learner,
                relationships: HashMap::new(),
            },
        ),
    ];
    assert_eq!(probability_of_quest_success(5, &heros_avg_3), Percent(30));
    assert_eq!(probability_of_quest_success(4, &heros_avg_3), Percent(50));
    assert_eq!(probability_of_quest_success(3, &heros_avg_3), Percent(70));
    assert_eq!(probability_of_quest_success(2, &heros_avg_3), Percent(90));
    assert_eq!(probability_of_quest_success(1, &heros_avg_3), Percent(110));

    let heros_avg_fractional = [
        (
            &LevelState {
                level: 3,
                exp: 0,
                exp_to_next: 100,
            },
            &Person {
                personality: Personality::Friendly,
                relationships: HashMap::new(),
            },
        ),
        (
            &LevelState {
                level: 2,
                exp: 0,
                exp_to_next: 100,
            },
            &Person {
                personality: Personality::ResultOriented,
                relationships: HashMap::new(),
            },
        ),
        (
            &LevelState {
                level: 5,
                exp: 0,
                exp_to_next: 100,
            },
            &Person {
                personality: Personality::Learner,
                relationships: HashMap::new(),
            },
        ),
    ];
    assert_eq!(
        probability_of_quest_success(4, &heros_avg_fractional),
        Percent(56)
    );
    assert_eq!(
        probability_of_quest_success(3, &heros_avg_fractional),
        Percent(76)
    );
    assert_eq!(
        probability_of_quest_success(2, &heros_avg_fractional),
        Percent(96)
    );
}

fn complete_quest_assign_exp(
    mut ev_quest_complete: EventReader<QuestCompleteEvent>,
    mut heroes_query: Query<&mut LevelState, With<Hero>>,
) {
    for event in ev_quest_complete.read() {
        for hero in &event.heroes {
            if let Ok(mut level_state) = heroes_query.get_mut(*hero) {
                level_state.exp += event.exp_reward;
            }
        }
    }
}

#[test]
fn complete_quest_assign_exp_increments_hero_exp() {
    let mut app = App::new();
    app.add_event::<QuestCompleteEvent>();
    app.add_systems(Update, complete_quest_assign_exp);
    // Add a hero with initial exp
    let hero_entity = app
        .world_mut()
        .spawn(HeroBundle {
            marker: Hero,
            level: LevelState {
                level: 1,
                exp: 50,
                exp_to_next: 100,
            },
            class: HeroClass::Warrior,
            person: Person {
                personality: Personality::Friendly,
                relationships: HashMap::new(),
            },
        })
        .id();
    // Add a QuestCompleteEvent with exp reward
    app.world_mut()
        .resource_mut::<Events<QuestCompleteEvent>>()
        .send(QuestCompleteEvent {
            quest_description: QuestDescription {
                difficulty_level: 1,
                turns_to_complete: 5,
                exp_reward: 50,
                gold_reward: 100,
                item_reward: None,
                turns_to_expiry: 10,
            },
            heroes: vec![hero_entity],
            success_probability: Percent(100),
            is_successful: true,
            exp_reward: 50,
            gold_reward: 100,
        });
    // Run the system
    app.update();
    // Check that the hero's exp was incremented
    let level_state = app.world().get::<LevelState>(hero_entity).unwrap();
    assert_eq!(level_state.exp, 100);
}

fn complete_quest_updates_guild(
    mut ev_quest_complete: EventReader<QuestCompleteEvent>,
    mut guild: ResMut<Guild>,
) {
    for event in ev_quest_complete.read() {
        if event.is_successful {
            guild.gold += event.gold_reward;
        }
    }
}

fn complete_quest_send_notification(
    mut ev_quest_complete: EventReader<QuestCompleteEvent>,
    mut ev_notify: EventWriter<NotificationEvent>,
) {
    for event in ev_quest_complete.read() {
        let success_str = if event.is_successful {
            "successful"
        } else {
            "failed"
        };
        ev_notify.write(NotificationEvent(format!(
            "Quest completed: {}. Heroes: {:?}, Exp Reward: {}, Gold Reward: {}, Success Probability: {:?}",
            success_str, event.heroes, event.exp_reward, event.gold_reward, event.success_probability
        )));
    }
}

#[test]
fn complete_quest_updates_guild_gold_only_on_success() {
    let mut app = App::new();
    app.init_resource::<Guild>();
    app.add_event::<QuestCompleteEvent>();
    app.add_systems(Update, complete_quest_updates_guild);
    // Add a QuestCompleteEvent with gold reward
    app.world_mut()
        .resource_mut::<Events<QuestCompleteEvent>>()
        .send(QuestCompleteEvent {
            quest_description: QuestDescription {
                difficulty_level: 1,
                turns_to_complete: 5,
                exp_reward: 50,
                gold_reward: 100,
                item_reward: None,
                turns_to_expiry: 10,
            },
            heroes: vec![],
            success_probability: Percent(100),
            is_successful: true,
            exp_reward: 50,
            gold_reward: 100,
        });
    // Run the system
    app.update();
    // Check that the guild's gold was incremented
    let guild = app.world().resource::<Guild>();
    assert_eq!(guild.gold, 100);

    // Add a QuestCompleteEvent which failed
    app.world_mut()
        .resource_mut::<Events<QuestCompleteEvent>>()
        .send(QuestCompleteEvent {
            quest_description: QuestDescription {
                difficulty_level: 1,
                turns_to_complete: 5,
                exp_reward: 50,
                gold_reward: 100,
                item_reward: None,
                turns_to_expiry: 10,
            },
            heroes: vec![],
            success_probability: Percent(0),
            is_successful: false,
            exp_reward: 50,
            gold_reward: 50,
        });
    // Run the system again
    app.update();
    // Check that the guild's gold was not incremented
    let guild = app.world().resource::<Guild>();
    assert_eq!(guild.gold, 100); // Still 100, since the quest failed
}

#[test]
fn complete_quest_sends_notification() {
    let mut app = App::new();
    app.add_event::<QuestCompleteEvent>();
    app.add_event::<NotificationEvent>();
    app.add_systems(Update, complete_quest_send_notification);

    // Create a Hero
    let hero_entity = app
        .world_mut()
        .spawn(HeroBundle {
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
        })
        .id();

    // Add a QuestCompleteEvent
    app.world_mut()
        .resource_mut::<Events<QuestCompleteEvent>>()
        .send(QuestCompleteEvent {
            quest_description: QuestDescription {
                difficulty_level: 1,
                turns_to_complete: 5,
                exp_reward: 50,
                gold_reward: 100,
                item_reward: None,
                turns_to_expiry: 10,
            },
            heroes: vec![hero_entity],
            success_probability: Percent(100),
            is_successful: true,
            exp_reward: 50,
            gold_reward: 100,
        });
    // Run the system
    app.update();
    // Check that a notification was sent
    let notification_events = app.world().resource::<Events<NotificationEvent>>();
    let mut reader = notification_events.get_cursor();
    let notification = reader.read(notification_events).next().unwrap();
    assert_eq!(
        notification.0,
        format!(
            "Quest completed: successful. Heroes: [{:?}], Exp Reward: 50, Gold Reward: 100, Success Probability: Percent(100)",
            hero_entity
        )
    );
}

// Heros level up when gaining enough experience

// Update hero opinions on quest ends

// TODO: incorporate hero opinions into quest success probability

// Periodically generate new quests

// Periodically generate new available heroes, with option of hiring them

// Heroes salary removed from guild gold every turn