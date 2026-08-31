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

    def test_gitlab_pipeline_name_is_an_optional_filter(self) -> None:
        base = {
            "DEMO_GITLAB_PROJECT": "romeo.cavazza/opswarden-demo",
            "DEMO_GITLAB_BRANCH": "demo/ci-failure",
        }
        self.assertEqual(
            demo.gitlab_trigger_config(base),
            {
                "repository": "romeo.cavazza/opswarden-demo",
                "branch": "demo/ci-failure",
                "conclusion": "failed",
            },
        )
        self.assertEqual(
            demo.gitlab_trigger_config({**base, "DEMO_GITLAB_PIPELINE": "named"})["workflow"],
            "named",
        )

    def test_psql_variables_are_separate_arguments(self) -> None:
        self.assertEqual(
            demo.Database.variable_args({"team_id": "one", "manager_id": "two"}),
            ["-v", "team_id=one", "-v", "manager_id=two"],
        )

    def test_fresh_local_team_uses_the_configured_id(self) -> None:
        class FakeDatabase:
            def __init__(self) -> None:
                self.user_ids = iter(
                    [
                        "10000000-0000-4000-8000-000000000001",
                        "10000000-0000-4000-8000-000000000002",
                        "10000000-0000-4000-8000-000000000003",
                        "10000000-0000-4000-8000-000000000004",
                    ]
                )
                self.executed_variables: dict[str, str] | None = None

            def query(self, sql: str, _variables: dict[str, str]) -> list[str]:
                if "select id from users" in sql:
                    return [next(self.user_ids)]
                return []

            def execute(self, _sql: str, variables: dict[str, str]) -> None:
                self.executed_variables = variables

        configured_team_id = "39aa8884-22cc-4764-a9e7-7df7c7619ba6"
        database = FakeDatabase()
        config = {
            "DEMO_LOCAL_TEAM_ID": configured_team_id,
            "DEMO_LOCAL_TEAM_NAME": "OpsWarden Demo",
            "DEMO_LOCAL_MANAGER_EMAIL": "manager@example.com",
            "DEMO_RESPONDER_EMAIL": "responder@example.com",
            "DEMO_OBSERVER_EMAIL": "observer@example.com",
            "DEMO_CONTRACTOR_EMAIL": "contractor@example.com",
        }

        variables = demo.identity_variables(config, database, "local")

        self.assertEqual(variables["team_id"], configured_team_id)
        self.assertEqual(database.executed_variables["team_id"], configured_team_id)

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

    def test_seed_locale_is_validated_and_persisted_for_every_demo_identity(self) -> None:
        class FakeDatabase:
            def __init__(self) -> None:
                self.sql = ""
                self.variables: dict[str, str] = {}

            def execute(self, sql: str, variables: dict[str, str]) -> None:
                self.sql = sql
                self.variables = variables

        identities = {
            "manager_id": "manager",
            "responder_id": "responder",
            "observer_id": "observer",
            "contractor_id": "contractor",
        }
        database = FakeDatabase()
        demo.persist_demo_locale({"DEMO_LOCALE": "fr"}, database, "local", identities)
        self.assertIn("update users set locale", database.sql)
        self.assertEqual(database.variables["locale"], "fr")

        with self.assertRaisesRegex(demo.DemoError, "either en or fr"):
            demo.persist_demo_locale({"DEMO_LOCALE": "de"}, database, "local", identities)

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
