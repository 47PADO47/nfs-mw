### How numbers are written (docs/specs/localization.md section 4.3).

# Between each three digits of a whole number: 20,000.
group-separator = ,

# A place in a race or a list: 1st, 2nd, 3rd, 4th.
format-ordinal = { NUMBER($n, type: "ordinal") ->
    [one] { $n }st
    [two] { $n }nd
    [few] { $n }rd
   *[other] { $n }th
}

format-percent = { $n }%
