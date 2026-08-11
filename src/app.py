"""Prometheusカスタムエクスポーターを提供します.

メインのエントリポイントとして、CLIで起動できる `exporter` 関数を含んでいます。
"""

from functools import cache

import click
from flask import Flask, Response

from config import get_optional_env_var, get_required_env_var
from switchbot import Switchbot, SwitchbotMetrics


def generate_prometheus_response_text(metrics: SwitchbotMetrics) -> str:
    """Prometheusのメトリクスのレスポンステキストを生成します.

    Args:
        metrics (dict): メトリクスデータ。

    Returns:
        str: Promtheusに表示するテキスト。

    """
    response_text = (
        "# HELP switchbot_device_battery SwitchBot Battery level\n"
        "# TYPE switchbot_device_battery gauge\n"
    )
    for device_id, battery in metrics.batteries.items():
        labels = (
            f'device_id="{device_id}",'
            f'device_name="{metrics.escape_device_names[device_id]}"'
        )
        response_text += f"switchbot_device_battery{{{labels}}} {battery}\n"

    response_text += (
        "# HELP switchbot_device_humidity SwitchBot Humidity\n"
        "# TYPE switchbot_device_humidity gauge\n"
    )
    for device_id, humidity in metrics.humidities.items():
        labels = (
            f'device_id="{device_id}",'
            f'device_name="{metrics.escape_device_names[device_id]}"'
        )
        response_text += f"switchbot_device_humidity{{{labels}}} {humidity}\n"

    response_text += (
        "# HELP switchbot_device_temperature SwitchBot Temperature\n"
        "# TYPE switchbot_device_temperature gauge\n"
    )
    for device_id, temperature in metrics.temperatures.items():
        labels = (
            f'device_id="{device_id}",'
            f'device_name="{metrics.escape_device_names[device_id]}"'
        )
        response_text += f"switchbot_device_temperature{{{labels}}} {temperature}\n"

    response_text += (
        "# HELP switchbot_device_co2 SwitchBot CO2\n# TYPE switchbot_device_co2 gauge\n"
    )
    for device_id, co2 in metrics.co2s.items():
        labels = (
            f'device_id="{device_id}",'
            f'device_name="{metrics.escape_device_names[device_id]}"'
        )
        response_text += f"switchbot_device_co2{{{labels}}} {co2}\n"

    response_text += (
        "# HELP switchbot_device_voltage SwitchBot Voltage\n"
        "# TYPE switchbot_device_voltage gauge\n"
    )
    for device_id, voltage in metrics.voltages.items():
        labels = (
            f'device_id="{device_id}",'
            f'device_name="{metrics.escape_device_names[device_id]}"'
        )
        response_text += f"switchbot_device_voltage{{{labels}}} {voltage}\n"

    response_text += (
        "# HELP switchbot_device_weight SwitchBot Weight\n"
        "# TYPE switchbot_device_weight gauge\n"
    )
    for device_id, weight in metrics.weights.items():
        labels = (
            f'device_id="{device_id}",'
            f'device_name="{metrics.escape_device_names[device_id]}"'
        )
        response_text += f"switchbot_device_weight{{{labels}}} {weight}\n"

    response_text += (
        "# HELP switchbot_device_electric_current SwitchBot ElectricCurrent\n"
        "# TYPE switchbot_device_electric_current gauge\n"
    )
    for device_id, electric_current in metrics.electric_currents.items():
        labels = (
            f'device_id="{device_id}",'
            f'device_name="{metrics.escape_device_names[device_id]}"'
        )
        response_text += (
            f"switchbot_device_electric_current{{{labels}}} {electric_current}\n"
        )

    return response_text.strip()


@cache
def get_switchbot() -> Switchbot:
    """環境設定から共有SwitchBot clientを生成します."""
    return Switchbot(
        api_token=get_required_env_var("SWITCHBOT_API_TOKEN"),
        api_secret=get_required_env_var("SWITCHBOT_API_SECRET"),
        cache_dir=str(get_optional_env_var("CACHE_DIR", "/tmp/switchbot", str)),
        cache_expire_second=int(get_optional_env_var("CACHE_EXPIRE_SECOND", 600, int)),
        delay_second=float(get_optional_env_var("DELAY_SECOND", 1, float)),
    )


def create_app(switchbot_client: Switchbot | None = None) -> Flask:
    """認証情報を遅延読込するFlask applicationを生成します."""
    flask_app = Flask(__name__)

    @flask_app.route("/metrics", methods=["GET"])
    def http_metrics() -> Response:
        """PrometheusのメトリクスのHTTPリクエストを処理します."""
        client = switchbot_client or get_switchbot()
        devices = client.fetch_devices()
        metrics = client.fetch_metrics(devices)
        response_text = generate_prometheus_response_text(metrics)
        return Response(response_text, content_type="text/plain; charset=utf-8")

    return flask_app


app = create_app()


@click.group()
def cli() -> None:
    """SwitchBotデバイスと対話するためのコマンド群."""
    pass


@click.command()
def devices() -> None:
    """デバイスのリストを取得して表示します."""
    devices = get_switchbot().fetch_devices()
    click.echo(devices)


@click.command()
@click.argument("device_id")
def device_status(device_id: str) -> None:
    """指定されたデバイスのステータスを取得して表示します."""
    status = get_switchbot().fetch_device_status(device_id)
    click.echo(status)


@click.command()
def metrics() -> None:
    """Prometheusのメトリクスを取得して表示します."""
    client = get_switchbot()
    devices = client.fetch_devices()
    metrics = client.fetch_metrics(devices)
    response_text = generate_prometheus_response_text(metrics)
    click.echo(response_text)


@click.command()
def exporter() -> None:
    """Prometheus用のカスタムエクスポーターを起動します."""
    server_port = int(get_optional_env_var("SERVER_PORT", 9171, int))
    app.run(host="0.0.0.0", port=server_port)


cli.add_command(devices)
cli.add_command(device_status)
cli.add_command(metrics)
cli.add_command(exporter)

if __name__ == "__main__":
    cli()
