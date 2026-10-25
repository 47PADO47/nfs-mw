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

## The Custom HUD's contextual layouts (ui/theme/custom: free_roam, race, pursuit, condition, words). Headings are
## shown in capitals.

# What Interact does on the entry card, after the key that does it.
hud-entry-action-go-in = Go in
hud-entry-action-leave = Leave
# The heading of a discovery chip for a place that is not a race.
hud-discovery-nearby = Nearby
# The free-roam extras line: $cash and $bounty are whole numbers with digit grouping, $distance a distance with its unit.
hud-extras-cash = ${ $cash }
hud-extras-bounty = Bounty { $bounty }
hud-extras-route = GPS { $distance }
# Distances: $value is a number, already rounded.
hud-distance-m = { $value } m
hud-distance-km = { $value } km
hud-distance-ft = { $value } ft
hud-distance-mi = { $value } mi
hud-race-type-sprint = Sprint
hud-race-type-circuit = Circuit
hud-race-type-drag = Drag
hud-race-type-knockout = Lap knockout
hud-race-type-tollbooth = Tollbooth
hud-race-type-speed-trap = Speedtrap
hud-race-type-challenge = Challenge
hud-race-type-race = Race
# The gap to the nearest rival at the latest checkpoint both passed: $seconds is unsigned, two decimals. Never a sign:
# the word says the side.
hud-gap-ahead = { $seconds } s AHEAD
hud-gap-behind = { $seconds } s BEHIND
hud-gap-level = LEVEL
# The race strip's headings.
hud-strip-position = Pos
hud-strip-lap = Lap
hud-strip-final-lap = Final lap
# A checkpoint race's gate.
hud-strip-checkpoint = CP
# The race intro card.
hud-intro-laps = { $laps ->
    [one] { $laps } lap
   *[other] { $laps } laps
}
hud-intro-opponents = { $n ->
    [one] { $n } opponent
   *[other] { $n } opponents
}
# The pursuit panel's status, a word beside its icon.
hud-pursuit-status-active = Active
hud-pursuit-status-search = Search
hud-pursuit-status-cooldown = Cooldown
hud-pursuit-heat = Heat
# $bounty is a whole number with digit grouping.
hud-pursuit-bounty = Bounty { $bounty }
# Where the police last saw the player, $secs whole seconds ago.
hud-pursuit-last-seen = Last seen { $secs } s ago
# $cost is a whole number with digit grouping.
hud-pursuit-cost = Cost { $cost }
# The cop cars that joined the chase and those wrecked.
hud-pursuit-units = Units { $engaged } / { $wrecked } wrecked
hud-pursuit-hiding = Hiding
hud-pursuit-escaped-card = Escaped
hud-pursuit-busted-card = Busted
# The vehicle condition chip.
hud-condition-damaged = Damaged
hud-condition-disabled = Vehicle disabled
hud-condition-tire-blown = Tyre blown
hud-condition-tire-punctured = Tyre punctured
hud-condition-heavy-damage = Heavy damage
hud-wheel-front-left = Front left
hud-wheel-front-right = Front right
hud-wheel-rear-left = Rear left
hud-wheel-rear-right = Rear right
# The count badge of a notice or a chip that repeats: $count is how many times it came.
hud-notify-count = ×{ $count }
# The feedback lane's bounty chip: $amount is the bounty gained, already written with the language's digit grouping.
hud-feedback-bounty-gain = +{ $amount }

## The Custom HUD's world markers (ui/theme/custom/markers/draw.rs)

# $gate is the next gate's number, $gates the gates of the race, $distance the distance to it (hud-marker-metres...).
hud-marker-gate = { $gate }/{ $gates }  { $distance }
# A race start, by its career event id (10.2.1).
hud-marker-race = Race { $event }
# Distances: $m whole metres, $km kilometres to a tenth, $ft whole feet, $mi miles to a tenth.
hud-marker-metres = { $m } m
hud-marker-kilometres = { $km } km
hud-marker-feet = { $ft } ft
hud-marker-miles = { $mi } mi
