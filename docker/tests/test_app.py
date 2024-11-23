import unittest
from unittest.mock import MagicMock, patch

from app import generate_prometheus_response_text
from switchbot import SwitchbotMetrics


class TestApp(unittest.TestCase):

    def setUp(self):
        pass

    def test_generate_prometheus_response_text(self):
        metrics = SwitchbotMetrics(
            escape_device_names={
                "123": "Test Bot",
            },
            batteries={
                "123": 100,
            },
        )

        response_text = generate_prometheus_response_text(metrics)
        self.assertTrue(isinstance(response_text, str))
