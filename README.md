# Running this

Follow setup instructions to install Rust and bevy: https://bevyengine.org/learn/quick-start/getting-started/setup/

Run command:
`RUST_LOG="warn,hero_guild_bevy=debug" cargo run --features bevy/dynamic_linking`

# The Game

Parties take 3 heroes: a warrior, a tank, and a support (or healer?)

## Heroes
Heroes have a level of 1-10, a class (warrior/tank/support), and personality which determines how they update their opinion of others.
Opinions of others are a scalar value, ranging from -5 (hate) to +5 (deep affection).
Heroes also have a single equipment slot.

## Personalities
1. +1 opinion of party members after questing together, regardless of outcome. Decays over time (to neutral?)
2. +1 opinion of party members if successful, -1 if not
3. Moves toward the other person's opinion of them
4. -2 opinion of party members if they get injured, +1 otherwise.
5. +1 opinion of anyone stronger, -1 of anyone weaker
6. +1 opinion of anyone weaker, -1 of anyone stronger


## Success rates
Quests have a level 1-10.
Hero's level, and how well they work together, determine likelihood of success. 
- by default, 3 heroes with the same level as the quest have 70% success rate, and 50% chance to avoid any injuries
- if they all have maxed relationship, its 90% success rate, and 70% to avoid any injuries
- maxing their equipment as well can provide another 10%, making 100% success possible. There will still be a 20% possibility of an injury.
- If maximally negative relationships, success is 50%, chance to avoid injury 30%.
- If a party of level 5 heroes attempts a level 4 quest, up success rates 20%. Going up a level, drop rates 20%.
- Higher chance of injury also means higher chance of bad injury.
- With a party of different levels or strengths, use the average. A level of +1 cancels a level of -1. A single high level hero cannot carry the party, because teamwork is critical.

## Injuries
- Light injury - unable to go on quests for 1 week
- Heavy injury - unable to go on quests for 1 month
- Permanent injury - unable to go on quests for 1 month, then receive permanent debuff of -20% 
- Death

## Quests
Each quest has
- difficulty level
- time to complete
- hero experience (awarded even on failure)
- gold reward
- reputation experience reward
- item reward
- time to expiry

## Items
Heroes can provide themselves with whatever basic equipment they need, but a legendary item can increase their effectiveness by 10%. Most of these items are exclusive to one class. Some particularly special items may be more flexible.

Other items remain in the guild's possession, and affect the quests that appear, essentially unlocking more, or better, or higher level quests.

## Game loop
The guild also has a reputation level. The goal of the game is to reach level 10.
Going broke means game over.
Each hero must be paid a salary. For now lets assume 10-20 gold per month, depending on level. 
At the beginning of every month, new quests will appear.
Occasionally, new heroes will appear and request to join the guild. Their level will be affected by the guild's reputation level. They will require a signing bonus (ie they must be bought).
You can let go of a hero you can't afford to pay. (There should be a hit to reputation.)
The primary choices are: how to put heroes together to optimize the parties, wisely choosing which quests to attempt, and growing without overextending.

## Future plans
- A system where you have to buy supplies to feed travelling heroes. 
- Travelling merchants who offer varying prices on food and items, or even may buy items.
- negotiated salaries
- individual happiness levels for heroes, with the threat of quitting
- heroes gain skills which make thing more complicated
- quests which unlock different quest chains
