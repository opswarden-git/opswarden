#!/usr/bin/env python3
"""Fast unit tests for the guarded presentation CLI."""

from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
import unittest


SPEC = importlib.util.spec_from_file_location("opswarden_demo", Path(__file__).with_name("demo.py"))
assert SPEC and SPEC.loader
demo = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(demo)


class DemoCliTests(unittest.TestCase):
    def test_target_value_prefers_target_override(self) -> None:
        config = {"DEMO_TEAM_NAME": "shared", "DEMO_LOCAL_TEAM_NAME": "local"}
        self.assertEqual(demo.target_value(config, "local", "TEAM_NAME"), "local")
        self.assertEqual(demo.target_value(config, "production", "TEAM_NAME"), "shared")

    def test_local_refuses_public_origin(self) -> None:
        with self.assertRaisesRegex(demo.DemoError, "refuses non-local"):
            demo.api_origin({"DEMO_LOCAL_API_ORIGIN": "https://api.opswarden.dev"}, "local")

    def test_production_requires_public_https(self) -> None:
        with self.assertRaisesRegex(demo.DemoError, "public HTTPS"):
            demo.api_origin({"DEMO_PRODUCTION_API_ORIGIN": "http://localhost:8080"}, "production")

    def test_production_confirmation_is_operation_specific(self) -> None:
        args = argparse.Namespace(target="production", confirm="SEED_PRODUCTION")
        demo.require_confirmation(args, "seed")
        with self.assertRaisesRegex(demo.DemoError, "DESEED_PRODUCTION"):
            demo.require_confirmation(args, "deseed")

    def test_local_confirmation_is_not_required(self) -> None:
        args = argparse.Namespace(target="local", confirm="")
        demo.require_confirmation(args, "seed")

    def test_psql_variables_are_separate_arguments(self) -> None:
        self.assertEqual(
            demo.Database.variable_args({"team_id": "one", "manager_id": "two"}),
            ["-v", "team_id=one", "-v", "manager_id=two"],
        )

    def test_presentation_delivery_ids_are_stable_and_provider_safe(self) -> None:
        self.assertEqual(
            demo.DEMO_DELIVERY_IDS["github"],
            "opswarden-demo-github-ci-failure-v1",
        )
        self.assertEqual(
            demo.DEMO_DELIVERY_IDS["alertmanager"],
            "sha256:d3acde02ddffbdb72461e544f436c5f6448d2c9a26aacedc53256310498722ff",
        )
        self.assertTrue(all(len(delivery_id) <= 255 for delivery_id in demo.DEMO_DELIVERY_IDS.values()))

    def test_wait_for_demo_runs_requires_every_rule_to_finish(self) -> None:
        class FakeDatabase:
            def __init__(self) -> None:
                self.responses = iter([["2"], ["running=1"], ["succeeded=2"]])

            def query(self, _sql: str, _variables: dict[str, str]) -> list[str]:
                return next(self.responses)

        self.assertEqual(
            demo.wait_for_demo_runs(FakeDatabase(), "team-1", timeout_seconds=1),
            {"succeeded": 2},
        )


if __name__ == "__main__":
    unittest.main()
