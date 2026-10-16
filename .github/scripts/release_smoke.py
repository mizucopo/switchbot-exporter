"""配布archiveの実行・設定・TLS/proxy・mock API・終了が検証されること。

Arrange: 共通controllerの配布物と外部接続を行わないloopback proxyが用意されること。
Act: 展開済みCLIからAPI取得とHTTP scrape、停止が実行されること。
Assert: checksum・既存メトリクス・設定優先順位・正常終了が確認されること。
"""

import argparse
import base64
import concurrent.futures
import ctypes
import hashlib
import hmac
import importlib.util
import json
import os
import shutil
import signal
import socket
import socketserver
import ssl
import subprocess
import sys
import tempfile
import threading
import time
import tomllib
from pathlib import Path
from types import SimpleNamespace
from typing import Any
from urllib import error, request

ROOT = Path(__file__).resolve().parents[2]
TOKEN = "distribution-mock-token"
# 公開済みの架空入力だけを使い、実環境の認証値は読み込まない。
PUBLIC_TEST_HMAC_INPUT = "distribution-mock-secret"
STATUS = {
    "battery": 88,
    "humidity": 55,
    "temperature": 23.5,
    "CO2": 700,
    "voltage": 100,
    "weight": 2.5,
    "electricCurrent": 0.4,
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def read_headers(stream: socket.socket) -> bytes:
    result = bytearray()
    while not result.endswith(b"\r\n\r\n"):
        part = stream.recv(1)
        require(bool(part) and len(result) < 16384, "incomplete mock request")
        result.extend(part)
    return bytes(result)


class MockProxy(socketserver.ThreadingTCPServer):
    daemon_threads = True

    def __init__(self, mode: str) -> None:
        self.mode = mode
        self.paths: list[str] = []
        self.failures: list[str] = []
        self.pending = threading.Event()
        self.resume = threading.Event()
        self.context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        self.context.load_cert_chain(
            ROOT / "tests/fixtures/tls/distribution.pem",
            ROOT / "tests/fixtures/tls/distribution-key.pem",
        )
        super().__init__(("127.0.0.1", 0), ProxyHandler)

    def handle_error(self, _request: Any, _client_address: Any) -> None:
        self.failures.append(str(sys.exception()))


class ProxyHandler(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        proxy = self.server
        assert isinstance(proxy, MockProxy)
        self.request.settimeout(10)
        require(
            read_headers(self.request).split(b"\r\n", 1)[0]
            == b"CONNECT api.switch-bot.com:443 HTTP/1.1",
            "unexpected proxy authority",
        )
        self.request.sendall(b"HTTP/1.1 200 Connection established\r\n\r\n")
        with proxy.context.wrap_socket(self.request, server_side=True) as stream:
            lines = read_headers(stream).decode().split("\r\n")
            method, path, _ = lines[0].split()
            require(method == "GET", "mock permits GET only")
            headers = {
                name.lower(): value.strip()
                for line in lines[1:]
                if line
                for name, value in [line.split(":", 1)]
            }
            timestamp = headers["t"]
            expected = base64.b64encode(
                hmac.new(
                    PUBLIC_TEST_HMAC_INPUT.encode(),
                    (TOKEN + timestamp).encode(),
                    hashlib.sha256,
                ).digest()
            ).decode()
            require(headers["authorization"] == TOKEN, "environment token precedence")
            require(headers["sign"] == expected, "dotenv secret or API signature")
            proxy.paths.append(path)
            if path == "/v1.1/devices":
                body: dict[str, Any] = {
                    "deviceList": [
                        {
                            "deviceId": "mock-device",
                            "deviceName": "配布テスト",
                            "deviceType": "MeterPro(CO2)",
                        }
                    ]
                }
                if proxy.mode == "partial":
                    body["deviceList"].insert(
                        0,
                        {
                            "deviceId": "failed-device",
                            "deviceName": "取得失敗",
                            "deviceType": "Meter",
                        },
                    )
            else:
                if path == "/v1.1/devices/failed-device/status":
                    require(proxy.mode == "partial", "partial failure fixture")
                    stream.sendall(
                        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n"
                        b"Connection: close\r\n\r\n"
                    )
                    return
                require(path == "/v1.1/devices/mock-device/status", "mock API path")
                body = STATUS
                if proxy.mode in ("timeout", "drain"):
                    proxy.pending.set()
                    proxy.resume.wait(timeout=10)
                if proxy.mode == "error":
                    stream.sendall(
                        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n"
                        b"Connection: close\r\n\r\n"
                    )
                    return
                if proxy.mode == "timeout":
                    return
            payload = json.dumps({"statusCode": 100, "body": body}).encode()
            stream.sendall(
                f"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n"
                f"Content-Length: {len(payload)}\r\nConnection: close\r\n\r\n".encode()
                + payload
            )


def signal_windows(pid: int) -> None:
    if sys.platform == "win32":
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.FreeConsole()
        require(kernel.AttachConsole(pid) != 0, "attach isolated exporter console")
        require(kernel.GenerateConsoleCtrlEvent(1, pid) != 0, "send Ctrl-Break")
    else:
        raise AssertionError("console control requires Windows")


def stop(process: subprocess.Popen[bytes]) -> None:
    if sys.platform == "win32":
        subprocess.run(
            [
                sys.executable,
                "-I",
                str(Path(__file__).resolve()),
                "--signal",
                str(process.pid),
            ],
            check=True,
            timeout=10,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
    else:
        process.send_signal(signal.SIGTERM)


def wait_listening(process: subprocess.Popen[bytes], port: int) -> None:
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        require(process.poll() is None, "exporter exited before startup")
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.1):
                return
        except OSError:
            time.sleep(0.05)
    raise AssertionError("exporter startup timed out")


def check_metrics(body: str) -> None:
    for suffix, value in (
        ("battery", "88"),
        ("humidity", "55"),
        ("temperature", "23.5"),
        ("co2", "700"),
        ("voltage", "100"),
        ("weight", "2.5"),
        ("electric_current", "0.4"),
    ):
        require(
            f'switchbot_device_{suffix}{{device_id="mock-device",'
            f'device_name="配布テスト"}} {value}' in body,
            "existing metrics contract: " + suffix,
        )


def exporter_smoke(binary: Path, directory: Path, mode: str) -> None:
    # Arrange
    case = directory / mode / "設定 with spaces"
    nested = case / "nested"
    nested.mkdir(parents=True)
    (case.parent / ".env").write_text("SWITCHBOT_API_SECRET=wrong-parent\n")
    ca = case / "テスト CA.pem"
    shutil.copyfile(ROOT / "tests/fixtures/tls/distribution-ca.pem", ca)
    (case / ".env").write_bytes(
        (
            "SERVER_PORT=invalid-dotenv-port\r\nSWITCHBOT_API_TOKEN=wrong-dotenv\r\n"
            f"SWITCHBOT_API_SECRET={PUBLIC_TEST_HMAC_INPUT}\r\n"
            f"REQUESTS_CA_BUNDLE='{ca}'\r\n"
            "CURL_CA_BUNDLE=missing-fallback.pem\r\nDELAY_SECOND=0\r\n"
            "CACHE_EXPIRE_SECOND=600\r\nAPI_TIMEOUT_SECOND="
            + ("0.2" if mode == "timeout" else "5")
            + "\r\n"
        ).encode()
    )
    env = {
        key: value
        for key, value in os.environ.items()
        if key.upper()
        not in {
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "NO_PROXY",
            "SWITCHBOT_API_TOKEN",
            "SWITCHBOT_API_SECRET",
            "SERVER_PORT",
            "CACHE_DIR",
            "CACHE_EXPIRE_SECOND",
            "DELAY_SECOND",
            "API_TIMEOUT_SECOND",
            "REQUESTS_CA_BUNDLE",
            "CURL_CA_BUNDLE",
        }
    }
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    env.update(HTTPS_PROXY="", NO_PROXY="")
    env["Server_Port" if os.name == "nt" else "SERVER_PORT"] = str(port)
    env["Switchbot_Api_Token" if os.name == "nt" else "SWITCHBOT_API_TOKEN"] = TOKEN
    flags = 0
    startup = None
    if sys.platform == "win32":
        flags = subprocess.CREATE_NEW_PROCESS_GROUP | subprocess.CREATE_NEW_CONSOLE
        startup = subprocess.STARTUPINFO()
        startup.dwFlags = subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = subprocess.SW_HIDE
    opener = request.build_opener(request.ProxyHandler({}))

    with MockProxy(mode) as proxy:
        proxy_thread = threading.Thread(target=proxy.serve_forever, daemon=True)
        proxy_thread.start()
        env["HTTPS_PROXY"] = f"http://127.0.0.1:{proxy.server_address[1]}"
        process = subprocess.Popen(
            [str(binary), "exporter"],
            cwd=nested,
            env=env,
            creationflags=flags,
            startupinfo=startup,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        def scrape(method: str = "GET") -> tuple[int, str]:
            query = request.Request(f"http://127.0.0.1:{port}/metrics", method=method)
            try:
                with opener.open(query, timeout=10) as response:
                    require(
                        "text/plain" in response.headers["Content-Type"], "HTTP type"
                    )
                    return response.status, response.read().decode()
            except error.HTTPError as failure:
                return failure.code, failure.read().decode()

        try:
            # Act
            wait_listening(process, port)
            if mode in ("drain", "timeout"):
                with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
                    pending = pool.submit(scrape)
                    require(proxy.pending.wait(timeout=10), "pending API request")
                    if mode == "drain":
                        stop(process)
                        time.sleep(0.1)
                        require(process.poll() is None, "pending scrape must drain")
                        proxy.resume.set()
                    status, body = pending.result(timeout=3)
                    if mode == "timeout":
                        require(not proxy.resume.is_set(), "timeout precedes mock EOF")
            else:
                status, body = scrape()
            # Assert
            if mode in ("success", "partial", "drain"):
                require(status == 200, f"mock HTTP status: {status}: {body}")
                check_metrics(body)
                require("failed-device" not in body, "failed device is omitted")
            else:
                require(
                    (status, body) == (500, "Internal Server Error"), "atomic error"
                )
            if mode == "success":
                check_metrics(scrape()[1])
                require(len(proxy.paths) == 2, "shared cache")
                require(scrape("HEAD") == (200, ""), "HEAD")
                require(scrape("OPTIONS") == (200, ""), "OPTIONS")
                for arguments in (
                    ["devices"],
                    ["device-status", "mock-device"],
                    ["metrics"],
                ):
                    output = subprocess.check_output(
                        [str(binary), *arguments], cwd=nested, env=env, timeout=10
                    ).decode()
                    if arguments == ["metrics"]:
                        check_metrics(output)
                    else:
                        require(bool(json.loads(output)), "CLI JSON")
            if mode == "partial":
                output = subprocess.run(
                    [str(binary), "metrics"],
                    cwd=nested,
                    env=env,
                    timeout=10,
                    capture_output=True,
                    check=True,
                )
                check_metrics(output.stdout.decode())
                require("failed-device" not in output.stdout.decode(), "CLI omission")
                require(
                    "metrics scrape omitted 1 target device(s)"
                    in output.stderr.decode(),
                    "CLI partial failure notice",
                )
            if mode != "drain":
                stop(process)
            _, stderr = process.communicate(timeout=10)
            require(process.returncode == 0, f"graceful shutdown: {stderr.decode()}")
            require(TOKEN not in stderr.decode(), "token is not logged")
            require(
                PUBLIC_TEST_HMAC_INPUT not in stderr.decode(), "secret is not logged"
            )
            if mode == "partial":
                require(
                    "metrics scrape omitted 1 target device(s)" in stderr.decode(),
                    "HTTP partial failure notice",
                )
            require(not proxy.failures, "mock failures: " + repr(proxy.failures))
            with socket.socket() as connection:
                require(
                    connection.connect_ex(("127.0.0.1", port)) != 0, "port released"
                )
        finally:
            proxy.resume.set()
            if process.poll() is None:
                process.kill()
            process.communicate(timeout=10)
            proxy.shutdown()
            proxy_thread.join(timeout=5)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--verify-existing", action="store_true")
    parser.add_argument("--signal", type=int)
    args = parser.parse_args()
    if args.signal is not None:
        signal_windows(args.signal)
        return
    require(args.target is not None and args.output is not None, "target and output")
    spec = importlib.util.spec_from_file_location(
        "release", ROOT / ".github/scripts/release.py"
    )
    assert spec is not None and spec.loader is not None
    release = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(release)
    policy = json.loads((ROOT / ".github/release.json").read_text(encoding="utf-8"))
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))[
        "package"
    ]["version"]
    sha = (
        subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT).decode().strip()
    )
    plan = {"rust": policy["publication"]["rust"], "release_tag": version}
    git = SimpleNamespace(root=str(ROOT))
    output = args.output.resolve()
    if not args.verify_existing:
        release.build_rust_distribution(git, plan, sha, args.target, output)
    artifact = output / args.target
    manifest = json.loads((artifact / "manifest.json").read_text())
    name = "switchbot-exporter" + (".exe" if os.name == "nt" else "")
    require(manifest["binaries"] == [name], "executable set")
    with tempfile.TemporaryDirectory(prefix="switchbot distribution ") as temporary:
        directory = Path(temporary)
        shutil.copytree(artifact, directory / ("rust-release-" + args.target))
        selected = {**plan, "rust": {**plan["rust"], "targets": [args.target]}}
        release.rust_deliverables(git, selected, sha, directory)
        extracted = directory / "extracted"
        extracted.mkdir()
        release.rust_extract(artifact / manifest["archive"], extracted)
        binary = extracted / name
        require(
            subprocess.check_output([str(binary), "--version"], timeout=10)
            .decode()
            .strip()
            == f"switchbot-exporter {version}",
            "version",
        )
        for mode in ("success", "partial", "error", "timeout", "drain"):
            exporter_smoke(binary, directory, mode)
    print(
        f"{args.target}: archive, SHA-256, config, mock API, "
        "TLS/proxy, partial metrics, HTTP and shutdown passed"
    )


if __name__ == "__main__":
    main()
