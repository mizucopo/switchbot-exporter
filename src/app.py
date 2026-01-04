"""Prometheusカスタムエクスポーターを提供します.

メインのエントリポイントとして、CLIで起動できる `exporter` 関数を含んでいます。
"""

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


# python-decouple を使用して環境変数を取得
# .env ファイルはプロジェクトルートを検索
SWITCHBOT_API_TOKEN = get_required_env_var("SWITCHBOT_API_TOKEN")
SWITCHBOT_API_SECRET = get_required_env_var("SWITCHBOT_API_SECRET")
SERVER_PORT: int = get_optional_env_var("SERVER_PORT", 9171, int)  # type: ignore
CACHE_DIR: str = get_optional_env_var("CACHE_DIR", "/tmp/switchbot", str)  # type: ignore
CACHE_EXPIRE_SECOND: int = get_optional_env_var("CACHE_EXPIRE_SECOND", 600, int)  # type: ignore
DELAY_SECOND: float = get_optional_env_var("DELAY_SECOND", 1, float)  # type: ignore

app = Flask(__name__)
switchbot = Switchbot(
    api_token=SWITCHBOT_API_TOKEN,
    api_secret=SWITCHBOT_API_SECRET,
    cache_dir=CACHE_DIR,
    cache_expire_second=CACHE_EXPIRE_SECOND,
    delay_second=DELAY_SECOND,
)


@app.route("/metrics", methods=["GET"])
def http_metrics() -> Response:
    """PrometheusのメトリクスのHTTPリクエストを処理します.

    Returns:
        Response: HTTPレスポンス。

    """
    devices = switchbot.fetch_devices()
    metrics = switchbot.fetch_metrics(devices)
    response_text = generate_prometheus_response_text(metrics)
    return Response(response_text, content_type="text/plain; charset=utf-8")


@click.group()
def cli() -> None:
    """SwitchBotデバイスと対話するためのコマンド群."""
    pass


@click.command()
def devices() -> None:
    """デバイスのリストを取得して表示します."""
    devices = switchbot.fetch_devices()
    click.echo(devices)


@click.command()
@click.argument("device_id")
def device_status(device_id: str) -> None:
    """指定されたデバイスのステータスを取得して表示します."""
    status = switchbot.fetch_device_status(device_id)
    click.echo(status)


@click.command()
def metrics() -> None:
    """Prometheusのメトリクスを取得して表示します."""
    devices = switchbot.fetch_devices()
    metrics = switchbot.fetch_metrics(devices)
    response_text = generate_prometheus_response_text(metrics)
    click.echo(response_text)


@click.command()
def exporter() -> None:
    """Prometheus用のカスタムエクスポーターを起動します."""
    app.run(host="0.0.0.0", port=SERVER_PORT)


cli.add_command(devices)
cli.add_command(device_status)
cli.add_command(metrics)
cli.add_command(exporter)

if __name__ == "__main__":
    cli()
