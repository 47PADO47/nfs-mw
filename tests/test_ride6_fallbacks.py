"""Tests for the surface fallbacks of tools/ride6-exporter (fallbacks_names.py, fallbacks_pick.py) with authored data.

No game data is needed. Run from the repo root:  python -m unittest discover tests
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools", "ride6-exporter"))

import fallbacks_names as names  # noqa: E402
import fallbacks_pick as pick  # noqa: E402


def found(vehicle, kind="glass", part="mirrors"):
    return pick.Found(vehicle, part, f"mat_{kind}", f"mat_{kind}_of_{vehicle}", {})


def part(vehicle, export, signature, kind="glass", part_id="mirrors"):
    return pick.Part(export, signature, {kind: found(vehicle, kind, part_id)})


class Names(unittest.TestCase):
    def test_a_kind_ignores_numbers_underscores_and_the_common_prefixes(self):
        self.assertEqual(names.kind("mat_livery_0_345_001"), "livery")
        self.assertEqual(names.kind("mat_mechanics_1"), "mechanics")
        self.assertEqual(names.kind("mat_mirror_view"), names.kind("mat_mirrorview"))
        self.assertEqual(names.kind("mat_lights"), names.kind("mat_light628"))

    def test_the_edge_and_the_tint_of_glass_are_glass(self):
        self.assertEqual(names.kind("BASE_glass_edge"), "glass")
        self.assertEqual(names.kind("mat_glasstint"), "glass")

    def test_a_placeholder_takes_the_kind_of_its_slot_and_has_none_without_one(self):
        self.assertEqual(names.surface_kind("WorldGridMaterial", "mat_glasstint"), "glass")
        self.assertIsNone(names.surface_kind("Default_Material", None))
        self.assertIsNone(names.surface_kind("Default_Material", "Default_Material"))
        self.assertIsNone(names.kind("mat_bike_lods"))

    def test_a_vehicle_id_is_split_into_make_model_and_year_without_the_variant_tokens(self):
        self.assertEqual(names.split("Yamaha_YZF-R1-RM-2008"), ("yamaha", "yzf-r1", 2008))
        self.assertEqual(names.split("Aprilia_RS-457-Ride-Fest-2024"), ("aprilia", "rs-457", 2024))

    def test_the_nearest_bike_is_the_same_make_with_the_longest_shared_model_start_then_the_nearest_year(self):
        target = "Kawasaki_Ninja-H2R-2019"
        others = ["Kawasaki_Ninja-ZX-10R-2021", "Ducati_Ninja-H2R-2019", "Kawasaki_Ninja-H2-2019", "Kawasaki_Ninja-H2-2015"]
        ordered = sorted(others, key=lambda other: names.distance(target, other))
        self.assertEqual(ordered, ["Kawasaki_Ninja-H2-2019", "Kawasaki_Ninja-H2-2015", "Kawasaki_Ninja-ZX-10R-2021", "Ducati_Ninja-H2R-2019"])

    def test_a_variant_is_as_near_as_the_bike_it_is_a_variant_of(self):
        self.assertEqual(names.distance("Yamaha_YZF-R1-EM-2008", "Yamaha_YZF-R1-2008")[:4], (False, -len("yzf-r1"), -1.0, 0))


class Choice(unittest.TestCase):
    def choose(self, library, kind="glass", own=None, signature=(("glass", 74),)):
        return pick.choose("Kawasaki_Ninja-H2R-2019", "mirrors", "mirror306", signature, kind, library, own or {}, [128, 128, 128])

    def test_the_same_mesh_beats_the_nearest_model(self):
        library = {
            "Kawasaki_Ninja-H2-2019": {"mirrors": part("Kawasaki_Ninja-H2-2019", "mirror305", (("glass", 70),))},
            "Suter_500-mmx-2017": {"mirrors": part("Suter_500-mmx-2017", "mirror306", (("glass", 74),))},
        }
        choice = self.choose(library)
        self.assertEqual((choice.rule, choice.found.vehicle), ("asset", "Suter_500-mmx-2017"))

    def test_the_same_part_of_the_same_make_beats_the_same_shape_of_another(self):
        library = {
            "Kawasaki_Ninja-H2-2019": {"mirrors": part("Kawasaki_Ninja-H2-2019", "mirror305", (("glass", 70),))},
            "Ducati_Panigale-V4-R-2019": {"mirrors": part("Ducati_Panigale-V4-R-2019", "mirror131", (("glass", 74),))},
        }
        choice = self.choose(library)
        self.assertEqual((choice.rule, choice.found.vehicle), ("family", "Kawasaki_Ninja-H2-2019"))

    def test_the_nearest_of_the_same_make_is_taken(self):
        library = {
            "Kawasaki_Ninja-ZX-10R-2021": {"mirrors": part("Kawasaki_Ninja-ZX-10R-2021", "mirror1", ())},
            "Kawasaki_Ninja-H2-2019": {"mirrors": part("Kawasaki_Ninja-H2-2019", "mirror2", ())},
        }
        self.assertEqual(self.choose(library).found.vehicle, "Kawasaki_Ninja-H2-2019")

    def test_the_same_shape_of_another_make_beats_the_bikes_own_other_surface(self):
        library = {"Ducati_Panigale-V4-R-2019": {"mirrors": part("Ducati_Panigale-V4-R-2019", "mirror131", (("glass", 74),))}}
        own = {"chassis": pick.Part("chassis1", (), {"glass": found("Kawasaki_Ninja-H2R-2019", "glass", "chassis")})}
        self.assertEqual(self.choose(library, own=own).rule, "shape")

    def test_the_bikes_own_surface_of_the_kind_beats_another_make_of_another_shape(self):
        library = {"Ducati_Panigale-V4-R-2019": {"mirrors": part("Ducati_Panigale-V4-R-2019", "mirror131", (("glass", 99),))}}
        own = {"chassis": pick.Part("chassis1", (), {"glass": found("Kawasaki_Ninja-H2R-2019", "glass", "chassis")})}
        choice = self.choose(library, own=own)
        self.assertEqual((choice.rule, choice.found.part), ("own", "chassis"))

    def test_another_make_is_the_last_donor_and_the_nearest_one_wins(self):
        library = {
            "Suzuki_GSX-R-600-2005": {"mirrors": part("Suzuki_GSX-R-600-2005", "m1", ())},
            "Honda_Ninja-H2-2019": {"mirrors": part("Honda_Ninja-H2-2019", "m2", ())},
        }
        choice = self.choose(library)
        self.assertEqual((choice.rule, choice.found.vehicle), ("make", "Honda_Ninja-H2-2019"))

    def test_a_donor_without_the_kind_is_not_a_donor(self):
        library = {"Kawasaki_Ninja-H2-2019": {"mirrors": part("Kawasaki_Ninja-H2-2019", "mirror305", (), kind="mechanics")}}
        self.assertEqual(self.choose(library).rule, "flat")

    def test_a_placeholder_takes_the_bikes_frame_then_its_paint(self):
        own = {
            "chassis": pick.Part("chassis1", (), {"livery": found("K", "livery", "chassis"), "mechanics": found("K", "mechanics", "chassis")}),
        }
        self.assertEqual(self.choose({}, kind=None, own=own).found.glb, "mat_mechanics")
        del own["chassis"].surfaces["mechanics"]
        self.assertEqual(self.choose({}, kind=None, own=own).found.glb, "mat_livery")

    def test_nothing_similar_is_a_flat_colour(self):
        choice = self.choose({}, kind="glass")
        self.assertEqual((choice.rule, choice.found, choice.colour), ("flat", None, [128, 128, 128]))


if __name__ == "__main__":
    unittest.main()
