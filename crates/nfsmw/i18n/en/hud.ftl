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

## The Custom HUD's telemetry cluster (ui/theme/custom/cluster): short labels, shown as written.

# The speed units beside the speed digits.
hud-cluster-kmh = KM/H
hud-cluster-mph = MPH
# The gear box: reverse and neutral (forward gears are their number).
hud-cluster-gear-reverse = R
hud-cluster-gear-neutral = N
hud-cluster-nitrous = N2O
# The speed breaker.
hud-cluster-speedbreaker = SB
hud-cluster-boost = BOOST
# The advanced row. $rpm is whole rpm; $psi and $g are numbers with one decimal; $load is a whole percentage.
hud-cluster-rpm = { $rpm } rpm
hud-cluster-psi = { $psi } psi
hud-cluster-load = { $load }%
hud-cluster-lateral-g = { $g } g

## The Custom HUD's notifications (hud/notify/catalog.rs)

# $level is the chase's new heat level, a whole number.
hud-notify-heat-up = Heat level { $level }
hud-notify-roadblock = Roadblock reported
hud-notify-roadblock-detail = Dispatch has a roadblock up
hud-notify-helicopter = Helicopter tracking you
hud-notify-contact-lost = Visual contact lost
hud-notify-contact-lost-detail = Stay out of sight
hud-notify-cooldown = Cooldown
hud-notify-cooldown-detail = Stay hidden to get away
hud-notify-escaped = Escaped
hud-notify-busted = Busted
# $number is the checkpoint just passed and $total the checkpoints of a lap.
hud-notify-checkpoint = Checkpoint { $number }/{ $total }
hud-notify-wrong-way = Wrong way
hud-notify-wrong-way-detail = Back to the start
# $lap is the lap just begun and $laps the laps of the race.
hud-notify-lap = Lap { $lap }/{ $laps }
# $place is the player's place written as an ordinal (2nd, 3rd).
hud-notify-position = Position { $place }
hud-notify-near-miss = Near miss
# $speed is the speed in whole units, $unit the game's word for them (KM/H, MPH).
hud-notify-speed-trap = Speed trap { $speed } { $unit }
hud-notify-speed-trap-slow = Speed trap: too slow
# $bounty is the milestone the bounty passed.
hud-notify-bounty = Bounty { $bounty }
hud-notify-tire = Tyre punctured
hud-notify-heavy-damage = Heavy damage
hud-notify-vehicle-critical = Vehicle disabled
