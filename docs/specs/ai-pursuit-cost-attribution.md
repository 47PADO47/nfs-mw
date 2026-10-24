# Pursuit cost and deployment attribution

Sources: reconstructed `AIPursuit::OnAttached`, `CalcTotalCostToState`,
`AIPerpVehicle::OnCausedCollision` and the pursuit statistics interface.
See `../provenance/ai-pursuit-driving.md` for source policy and evidence limits.

**[decomp]** Cost to state adds the following independently:

| Event | Cost |
|---|---:|
| Cop destroyed | 5000 |
| Helicopter attached | 2000 |
| Roadblock deployed | 500 |
| Cop damaged | 250 |
| Traffic car hit by the perpetrator | 500 |
| Roadblock prop deployed, including sawhorses | 250 |
| Helicopter spike strip | 225 |
| Ordinary ground cop attached | 250 |
| Support-class ground car attached | 450 |
| Property damage | Authored object cost |

Support-class names are `copsuv`, `copsuvl`, `copcross` and `copsporthench`.
Classification follows the attached car's model, independently of whether it
belongs to a support request. Thus a federal hench car in a normal wave is still
a support-class deployment for the cost page. Failed placement does not count.
An existing patrol that joins the pursuit counts when attached. Retirement does
not subtract deployment counts. Roadblock cars count only when recruited into
the pursuit, rather than merely placed at a roadblock.

**[decomp]** A direct traffic hit qualifies when relative contact speed exceeds
4 m/s and the perpetrator contributes more velocity into the collision normal
than the traffic car. Preserve signed normal components; absolute speeds cannot
distinguish being rear-ended from causing a rear-end collision.

Only property impacts that move or detach the object emit the cost notification;
immobile contacts and zero-cost objects do not add property damage counts.

**[adapter]** The remake reports one qualifying contact episode per traffic entity
and one property impact episode per object ID. Multiple primitives or successive
physics steps in continuous contact do not multiply the same impact. A separated
object may contribute a later impact. Original callback coalescing remains
unverified. Exact indirect two-second causality propagation is still open.

All counters survive pursuit termination until the next pursuit starts. Career
history copies them at termination, including events occurring on the final
physics step. The eight extra sums follow `career-rapsheet-2.md` section 2:
roadblocks, traffic hits, roadblock props, heli strips, ordinary cars, support
cars, property value, property count. Undeployed heli strips remain zero.
