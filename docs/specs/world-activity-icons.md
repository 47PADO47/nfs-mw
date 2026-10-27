# World activity icons

Original retail PC 1.3 stores a table of world-icon types, model hashes and
particle-group hashes. The type numbers and world model/particle pair are
cross-checked against the GIcon interface in the CC0 reference and the names
in the user's installed InGameA and ATTRIBUTES files. See provenance.

Sprint, circuit, drag, knockout, tollbooth, speed trap and rival use their
matching CHECKPOINT_GEOMETRY_*_00 models and fxgame_icongrp_* groups. Safe
houses, car lots and custom shops use SAFEHOUSE/safe, CARLOT/lot and
CHOPSHOP/chop respectively. Hiding spots use HIDECAR/hidecar. Pursuit breaker
icons have no model in the table and use the pursuit particle group.

The rewrite instantiates the stock models from InGameA and the installed
particle groups at its existing entry-zone positions. The marker position,
heading, activation and lifetime rules are not yet established from original
execution. In this initial approximation, icons are omitted during races and
cutscenes; hidden zones show during pursuit. The stock particle group and model
are independently optional, so a missing asset must not suppress the other.
