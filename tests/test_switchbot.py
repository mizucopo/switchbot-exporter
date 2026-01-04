import json
import unittest
from unittest.mock import MagicMock, patch

from switchbot import Switchbot, SwitchbotDevice


class TestSwitchbot(unittest.TestCase):
    def setUp(self) -> None:
        self.api_token = "test_token"
        self.api_secret = "test_secret"
        self.cache_expire_second = 0

        self.switchbot = Switchbot(
            self.api_token,
            self.api_secret,
            cache_expire_second=self.cache_expire_second,
        )

    # def test_real_fetch(self):
    #     devices = self.switchbot.fetch_devices()
    #     self.switchbot.fetch_device_status("3030F9CE2626")
    #     self.switchbot.fetch_metrics(devices)

    @patch("switchbot.requests.get")
    def test_fetch_devices(self, mock_get: MagicMock) -> None:
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.text = json.dumps(
            {
                "body": {
                    "deviceList": [
                        {
                            "deviceId": "123",
                            "deviceType": "Bot",
                            "deviceName": "Test Bot",
                        }
                    ]
                }
            }
        )
        mock_get.return_value = mock_response

        devices = self.switchbot.fetch_devices()
        self.assertEqual(len(devices), 1)
        self.assertEqual(devices[0].device_id, "123")
        self.assertEqual(devices[0].device_type, "Bot")
        self.assertEqual(devices[0].device_name, "Test Bot")

    @patch("switchbot.requests.get")
    def test_fetch_device_status(self, mock_get: MagicMock) -> None:
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.text = json.dumps(
            {
                "body": {
                    "battery": 100,
                    "humidity": 50,
                    "temperature": 22,
                    "CO2": 400,
                    "voltage": 3.7,
                    "weight": 70,
                    "electricCurrent": 0.5,
                }
            }
        )
        mock_get.return_value = mock_response

        status = self.switchbot.fetch_device_status("123")
        self.assertEqual(status["body"]["battery"], 100)
        self.assertEqual(status["body"]["humidity"], 50)
        self.assertEqual(status["body"]["temperature"], 22)
        self.assertEqual(status["body"]["CO2"], 400)
        self.assertEqual(status["body"]["voltage"], 3.7)
        self.assertEqual(status["body"]["weight"], 70)
        self.assertEqual(status["body"]["electricCurrent"], 0.5)

    @patch("switchbot.requests.get")
    def test_fetch_metrics(self, mock_get: MagicMock) -> None:
        def mock_get_side_effect(
            url: str, *_args: object, **_kwargs: object
        ) -> MagicMock:
            response = MagicMock()
            if url.endswith("/123/status"):
                response.status_code = 200
                response.text = json.dumps(
                    {
                        "body": {
                            "battery": 100,
                            "humidity": 50,
                            "temperature": 22,
                        }
                    }
                )
                return response
            elif url.endswith("/456/status"):
                response.status_code = 200
                response.text = json.dumps(
                    {
                        "body": {
                            "CO2": 400,
                            "voltage": 3.7,
                        }
                    }
                )
                return response
            else:
                response.status_code = 404
                response.text = "Not Found"
                return response

        mock_get.side_effect = mock_get_side_effect

        devices = [
            SwitchbotDevice(device_id="123", device_type="Bot", device_name="Test Bot"),
            SwitchbotDevice(
                device_id="456", device_type="Meter", device_name="Test Meter"
            ),
        ]

        metrics = self.switchbot.fetch_metrics(devices)
        self.assertEqual(metrics.escape_device_names["123"], "Test Bot")
        self.assertEqual(metrics.batteries["123"], 100)
        self.assertEqual(metrics.humidities["123"], 50)
        self.assertEqual(metrics.temperatures["123"], 22)
        self.assertFalse("123" in metrics.co2s)
        self.assertFalse("123" in metrics.voltages)
        self.assertFalse("123" in metrics.weights)
        self.assertFalse("123" in metrics.electric_currents)

        self.assertEqual(metrics.escape_device_names["456"], "Test Meter")
        self.assertEqual(metrics.co2s["456"], 400)
        self.assertEqual(metrics.voltages["456"], 3.7)
        self.assertFalse("456" in metrics.batteries)
        self.assertFalse("456" in metrics.humidities)
        self.assertFalse("456" in metrics.temperatures)
        self.assertFalse("456" in metrics.weights)
        self.assertFalse("456" in metrics.electric_currents)


if __name__ == "__main__":
    unittest.main()
