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

    def test_psql_variables_are_separate_arguments(self) -> None:
        self.assertEqual(
            demo.Database.variable_args({"team_id": "one", "manager_id": "two"}),
            ["-v", "team_id=one", "-v", "manager_id=two"],
        )


if __name__ == "__main__":
    unittest.main()
