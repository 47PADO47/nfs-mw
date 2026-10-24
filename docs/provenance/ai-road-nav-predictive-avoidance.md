# Predicted road-corridor cuts

Date: 2026-10-24.

Specification: [road navigation trail, section 5](../specs/ai-road-nav-trail.md).

The original PC executable was identified before a bounded inspection of its
avoidable hole-punching routine. Native instructions confirm that the first
cookie to cut is selected near a predicted encounter position. The position
includes the other vehicle's road speed times approach time, then subtracts
the combined vehicle extents. It is not selected around the other vehicle's
current body position alone. Both locally retained PC executables must have
matching inspected instruction bytes before reusing this native evidence.
The bounded closest-cookie helper also confirms its forward-crossing test,
distance selection and absence of an endpoint fallback. The hole-punching
routine confirms that nose points use road-forward extents. Its tail expression
has a coordinate discrepancy in the inspected PC code. The implementation
retains the existing specification's horizontal projected-tail adapter;
reproducing that raw PC expression remains outside this correction.

The CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) reference and
the existing trail specification supplied the routine's structure. The new
regression uses independently constructed straight-road cookies and moving
vehicles. No game traces or original executable code are included.

This evidence establishes the predicted longitudinal start of the cut. It
does not settle every side-choice, overlap, tail-length or formation-exclusion
rule in the larger avoidance routine. Those remain separate checks.
