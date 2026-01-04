"""Switchbot.

Switchbotは、SwitchBot APIと対話するための機能を提供します.

"""

import base64
import hashlib
import hmac
import json
import logging
import os
import time
from dataclasses import dataclass, field
from typing import TypedDict

import inflect
import inflection
import requests


@dataclass
class SwitchbotDevice:
    """SwitchbotDeviceは、Switchbotデバイスのデータを表すクラス.

    Attributes:
        device_id (str): デバイスの一意の識別子。
        device_type (str): デバイスの種類。
        device_name (str): デバイスの名前。

    """

    device_id: str
    device_type: str
    device_name: str


@dataclass
class SwitchbotMetrics:
    """SwitchbotMetricsは、SwitchBotデバイスのメトリクスを管理するデータクラス."""

    escape_device_names: dict[str, str] = field(default_factory=dict)
    batteries: dict[str, float] = field(default_factory=dict)
    humidities: dict[str, float] = field(default_factory=dict)
    temperatures: dict[str, float] = field(default_factory=dict)
    co2s: dict[str, float] = field(default_factory=dict)
    voltages: dict[str, float] = field(default_factory=dict)
    weights: dict[str, float] = field(default_factory=dict)
    electric_currents: dict[str, float] = field(default_factory=dict)


class SwitchbotResponseDevice(TypedDict):
    """SwitchbotResponseDeviceは、SwitchBot APIのデバイス情報を表す型."""

    deviceId: str
    deviceName: str
    deviceType: str


class SwitchbotResponseDevicesBody(TypedDict):
    """SwitchbotResponseDevicesBodyは、SwitchBot APIのデバイスリストの本体を表す型."""

    deviceList: list[SwitchbotResponseDevice]


class SwitchbotResponseDevices(TypedDict):
    """SwitchbotResponseDevicesは、SwitchBot APIのデバイスリストを表す型."""

    body: SwitchbotResponseDevicesBody


class SwitchbotResponseStatus(TypedDict):
    """SwitchbotResponseStatusは、SwitchBot APIのデバイスステータスを表す型."""

    body: dict[str, float]


class Switchbot:
    """Switchbotクラスは、SwitchBot APIと対話するための機能を提供します.

    Attributes:
        VERSION (str): クラスのバージョン情報。
        BASE_URL (str): SwitchBot APIのベースURL。
        DEVICE_TYPES (list): サポートされているデバイスの種類のリスト。
        METRICS (list): サポートされているメトリクスのリスト。
        api_token (str): API認証に使用されるトークン。
        api_secret (str): API認証に使用されるシークレット。
        cache_enable (bool): キャッシュ機能が有効かどうか。
        cache_dir (str): キャッシュファイルを保存するディレクトリ。
        cache_expire_second (int): キャッシュの有効期限（秒）。
        delay_second (float): APIリクエスト間の遅延時間（秒）。

    """

    VERSION = "1.0.0"
    BASE_URL = "https://api.switch-bot.com"
    DEVICE_TYPES = [
        "Bot",
        "Ceiling Light",
        "Color Bulb",
        "Contact Sensor",
        "Curtain",
        "Hub Mini",
        "Indoor Cam",
        "Meter",
        "MeterPro(CO2)",
        "Motion Sensor",
        "Plug Mini (JP)",
        "Remote",
    ]
    METRICS = [
        "battery",
        "humidity",
        "temperature",
        "CO2",
        "voltage",
        "weight",
        "electricCurrent",
    ]

    def __init__(
        self: "Switchbot",
        api_token: str,
        api_secret: str,
        cache_dir: str = "/tmp/switchbot",
        cache_expire_second: int = 600,
        delay_second: float = 1,
    ) -> None:
        """SwitchBotクラスをAPIトークンとシークレットで初期化します.

        Args:
            api_token (str): 認証用のAPIトークン。
            api_secret (str): 認証用のAPIシークレット。
            cache_dir (str): キャッシュファイルを保存するディレクトリ。
            cache_expire_second (int): キャッシュの有効期限（秒）。
            delay_second (float): APIリクエスト間の遅延時間（秒）。

        """
        self.logger = logging.getLogger(__name__)

        self.api_token = api_token
        self.api_secret = api_secret
        self.cache_dir = cache_dir
        self.cache_expire_second = cache_expire_second
        self.delay_second = delay_second

        self.cache_enable = self.cache_dir and self.cache_expire_second > 0

        if self.cache_enable:
            os.makedirs(self.cache_dir, exist_ok=True)

    def fetch_devices(self: "Switchbot") -> list[SwitchbotDevice]:
        """デバイスデータを取得し、構造化された形式で返します.

        Returns:
            list[SwitchbotDevice]: デバイスデータを含むリスト

        """
        devices = []
        data = self.__fetch_devices()
        for device in data["body"]["deviceList"]:
            devices.append(
                SwitchbotDevice(
                    device_id=device["deviceId"],
                    device_type=device["deviceType"],
                    device_name=device["deviceName"],
                )
            )
        return devices

    def fetch_device_status(
        self: "Switchbot", device_id: str
    ) -> SwitchbotResponseStatus:
        """指定されたデバイスIDのデバイスのステータスを取得し、辞書として返します.

        Args:
            device_id (str): ステータスを取得するデバイスのID

        Returns:
            dict: デバイスのステータスを含む辞書(SwitchBotAPIのレスポンス)

        """
        status_url = f"{self.BASE_URL}/v1.1/devices/{device_id}/status"
        response = self.__fetch_with_cache(status_url)
        device_status: SwitchbotResponseStatus = json.loads(response)
        return device_status

    def fetch_metrics(
        self: "Switchbot", devices: list[SwitchbotDevice]
    ) -> SwitchbotMetrics:
        """Fetch metrics for each device and return them in a structured format.

        Args:
            devices (list): The list of devices.

        Returns:
            SwitchbotMetrics: なにか書く

        """
        ifct = inflect.engine()
        metrics = SwitchbotMetrics()

        for device in devices:
            if device.device_type in self.DEVICE_TYPES:
                status = self.fetch_device_status(device.device_id)

                device_name = device.device_name.replace('"', '\\"')
                metrics.escape_device_names[device.device_id] = device_name

                for metric in self.METRICS:
                    plural_metric = ifct.plural(inflection.underscore(metric).lower())
                    if metric in status["body"]:
                        getattr(metrics, plural_metric)[device.device_id] = status[
                            "body"
                        ][metric]

        return metrics

    def __generate_signature(
        self: "Switchbot", token: str, secret: str
    ) -> tuple[str, str, str]:
        """トークンとシークレットに基づいて署名、タイムスタンプ、およびノンスを生成して返します.

        Args:
            token (str): 認証トークン
            secret (str): 署名を生成するために使用されるシークレット

        Returns:
            tuple[str, str, str]: 署名、タイムスタンプ、およびノンスを含むタプル

        """
        nonce = ""
        timestamp = str(int(round(time.time() * 1000)))
        string_to_sign = bytes(f"{token}{timestamp}{nonce}", "utf-8")
        secret_bytes = bytes(secret, "utf-8")
        sign = base64.b64encode(
            hmac.new(
                secret_bytes, msg=string_to_sign, digestmod=hashlib.sha256
            ).digest()
        ).decode("utf-8")
        return sign, timestamp, nonce

    def __create_request_header(
        self: "Switchbot", token: str, secret: str
    ) -> dict[str, str]:
        """トークンとシークレットに基づいてリクエストヘッダーを作成して返します.

        Args:
            token (str): 認証トークン
            secret (str): 署名を生成するために使用されるシークレット

        Returns:
            dict[str, str]: リクエストヘッダーのフィールドを含む辞書

        """
        sign, t, nonce = self.__generate_signature(token, secret)
        headers = {"Authorization": token, "sign": sign, "t": t, "nonce": nonce}
        return headers

    def __cache_filename_from_url(self: "Switchbot", url: str) -> str:
        """指定されたURLに基づいてキャッシュファイル名を生成します.

        この関数はURLを受け取り、SHA256を使用してURLをハッシュ化することで一意のファイル名に変換します
        生成されたファイル名は、そのURLに対応するキャッシュされたコンテンツを保存するために使用されます

        Args:
            url (str): キャッシュファイル名を生成するためのURL

        Returns:
            str: 生成されたキャッシュファイル名

        """
        sha256_hash = hashlib.sha256(url.encode()).hexdigest()
        return os.path.join(self.cache_dir, sha256_hash)

    def __save_to_cache(self: "Switchbot", url: str, content: str) -> None:
        """指定されたURLに対応するキャッシュファイルにコンテンツを保存します.

        この関数は、URLから派生したファイル名にコンテンツを書き込みます
        URLに対応するキャッシュファイルが既に存在する場合、それは上書きされます

        Args:
            url (str): コンテンツに関連付けられたURL
            content (str): キャッシュするコンテンツ

        """
        with open(self.__cache_filename_from_url(url), "w") as cache_file:
            cache_file.write(content)

    def __load_from_cache(self: "Switchbot", url: str) -> str | None:
        """指定されたURLに対応するキャッシュファイルからコンテンツを取得します.

        この関数は、指定されたURLに関連付けられたキャッシュファイルからコンテンツを読み取ろうとします
        キャッシュファイルが存在しない場合や、キャッシュされたコンテンツが期限切れの場合は、Noneを返します

        Args:
            url (str): キャッシュされたコンテンツを取得するためのURL

        Returns:
            str:
                利用可能で期限切れでない場合はキャッシュされたコンテンツ、
                それ以外の場合はNone

        """
        cache_file = self.__cache_filename_from_url(url)
        if not os.path.exists(cache_file):
            return None
        if time.time() - os.path.getmtime(cache_file) > self.cache_expire_second:
            return None
        with open(cache_file, "r") as f:
            return f.read()

    def __fetch_with_cache(self: "Switchbot", url: str) -> str:
        """キャッシュを利用してHTTP GETリクエストを実行します.

        指定されたURLに対して有効なキャッシュコンテンツが存在するかどうかを最初に確認します
        有効なキャッシュコンテンツが見つかった場合、それを返します。そうでない場合は、新しいGETリクエストを行います
        新しいGETリクエストからの成功したレスポンスはキャッシュに保存されます

        Args:
            url (str): キャッシュから取得するか、フェッチするURL

        Returns:
            str: HTTP GETリクエストのレスポンス

        Raises:
            requests.exceptions.RequestException: HTTPエラーが発生した場合

        """
        try:
            if self.cache_enable:
                cached_content = self.__load_from_cache(url)
                if cached_content:
                    return cached_content

            headers = self.__create_request_header(self.api_token, self.api_secret)
            response = requests.get(url, headers=headers)
            response.raise_for_status()
            if self.cache_enable:
                self.__save_to_cache(url, response.text)
            time.sleep(self.delay_second)
            return response.text

        except requests.exceptions.RequestException as e:
            self.logger.error(f"response error: {e}")
            raise

    def __fetch_devices(self: "Switchbot") -> SwitchbotResponseDevices:
        """デバイスのリストを取得し、辞書として返します.

        Returns:
            SwitchbotResponseDevices:
                デバイスのリストを含む辞書(SwitchBotAPIのレスポンス)

        """
        devices_url = f"{self.BASE_URL}/v1.1/devices"
        response = self.__fetch_with_cache(devices_url)
        device_list: SwitchbotResponseDevices = json.loads(response)
        return device_list
