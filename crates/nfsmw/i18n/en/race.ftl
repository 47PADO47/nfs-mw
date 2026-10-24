### race

## The banners over a race (scenes/world/race/flow.rs)

race-banner-checkpoint = Checkpoint { $number }/{ $total }
race-banner-lap = Lap { $lap }/{ $laps }
# A time bonus, whole seconds.
race-banner-bonus = +{ $seconds } s
# A speed trap passed: $speed in whole km/h, $unit the game's word for it (KM/H).
race-banner-speed-trap = +{ $speed } { $unit }

## Racers' names when nothing else names them (scenes/world/race/report.rs, start.rs)

race-racer-fallback = Racer { $n }
# The player in a race's results without a loaded profile.
race-player-fallback = player

## The pursuit milestones on the rewards screen (scenes/world/career/rewards.rs)

# $time is minutes and seconds (05:00).
race-milestone-chase-for = Chase for { $time }
race-milestone-escape-within = Escape within { $time }
race-milestone-bounty = { $n } bounty in one chase
race-milestone-cops-destroyed = { $n ->
    [one] { $n } police car destroyed
   *[other] { $n } police cars destroyed
}
race-milestone-cost-to-state = { $n } cost to state
race-milestone-roadblocks = { $n ->
    [one] { $n } roadblock dodged
   *[other] { $n } roadblocks dodged
}
race-milestone-cops-hit = { $n ->
    [one] { $n } police car hit
   *[other] { $n } police cars hit
}
race-milestone-infractions = { $n ->
    [one] { $n } infraction in one chase
   *[other] { $n } infractions in one chase
}
race-milestone-spike-strips = { $n ->
    [one] { $n } spike strip dodged
   *[other] { $n } spike strips dodged
}

## The Blacklist's rivals in the Remastered look (ui/theme/remastered/menu): shown in capitals.

race-rival-complete = Complete
race-rival-in-progress = In progress
race-rival-locked = Locked
race-rival-status = Status
race-rival-alias = Alias
race-rival-car = Car
race-rival-strength = Strength
