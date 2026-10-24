### hud

## The race card's result line (hud/race_card.rs)

hud-result-won = You won
# $place is the place written as an ordinal (2nd, 3rd).
hud-result-placed = You finished { $place }
hud-result-lost = Race lost
hud-result-challenge-complete = Challenge complete
hud-result-challenge-failed = Challenge failed
# The points of a speed-trap race, a whole number.
hud-race-points = Speed { $points }

## The end-of-pursuit card (hud/pursuit_end.rs)

hud-pursuit-escaped = You escaped
hud-pursuit-busted = Busted
hud-pursuit-cops-wrecked = Cops wrecked: { $n }
# $time is the chase's length (01:15.50).
hud-pursuit-time = Time: { $time }

## The radio's cards (hud/radio.rs, hud/trax.rs): shown in capitals after the time or the album.

hud-radio-paused = Paused

## The entry prompts of the world (scenes/world/entry.rs)

hud-entry-auto-shop = Auto shop
hud-entry-safe-house = Safe house
hud-entry-car-lot = Car lot
hud-entry-race = Race
hud-entry-interior = Interior
hud-entry-go-in = Press Enter to go in
hud-entry-leave = Press Enter to leave
hud-entry-entering-auto-shop = Entering auto shop
hud-entry-entering-safe-house = Entering safe house
hud-entry-entering-car-lot = Entering car lot
hud-entry-entering-race = Entering race
hud-entry-not-built = Not built yet
hud-entry-race-locked = Race locked
hud-entry-not-open = Not open yet

## A speed trap in free roam (scenes/world/career/speedtraps.rs)

# $speed is whole km/h, $unit the game's word for it (KM/H), $bounty the bounty it paid.
hud-speed-trap = Speed trap: { $speed } { $unit }  +{ $bounty } bounty

## The Remastered pursuit panel (ui/theme/remastered/pursuit): shown in capitals.

hud-pursuit-time-label = Pursuit time
hud-pursuit-called = Called
hud-pursuit-damaged = Damaged
hud-pursuit-wrecked = Wrecked
# The side of the meter where the player is getting away (the other side is the game's BUSTED).
hud-pursuit-escaping = Escaping

## The Custom theme's meters (ui/theme/custom/meters.rs): short labels, shown in capitals.

hud-meter-nitrous = NOS
hud-meter-boost = Boost
# The speed breaker.
hud-meter-speedbreaker = SB
