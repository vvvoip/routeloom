"""Print an allowlisted VPN summary; never echo scutil's credential-bearing options."""
import json
import plistlib
import re
import subprocess
import sys


def summarize(text):
    lines = text.splitlines()
    status = lines[0].strip() if lines else "Unavailable"
    allowed = {"Disconnected", "Connecting", "Connected", "Disconnecting", "Invalid", "Reasserting"}
    result = {"status": status if status in allowed else "Unavailable"}
    # Deliberately exclude StartMessage, SessionOptions, profile and all arbitrary text.
    for key in ("ConnectCount", "ConnectedCount", "DisconnectedCount", "MaxConnectTime", "NEStatus", "SessionState"):
        match = re.search(r"^\s*" + key + r" : ([0-9]{1,15})\s*$", text, re.MULTILINE)
        if match:
            result[key] = int(match[1])
    match = re.search(r"LastDisconnectError : <data> 0x([0-9a-fA-F]{1,65536})(?:\s|$)", text)
    if match:
        try:
            archive = plistlib.loads(bytes.fromhex(match[1]))
            objects = archive["$objects"]
            error = objects[archive["$top"]["root"].data]
            domain = objects[error["NSDomain"].data]
            code = error["NSCode"]
            if isinstance(domain, str) and re.fullmatch(r"[A-Za-z0-9_.-]{1,160}", domain) and type(code) is int:
                result["last_error"] = {"domain": domain, "code": code}
        except (ValueError, TypeError, KeyError, IndexError, AttributeError, OverflowError):
            result["last_error"] = {"decode": "unavailable"}
    return result


def main():
    if sys.platform != "darwin":
        raise SystemExit("Run this diagnostic on your Mac.")
    try:
        output = subprocess.run(
            ["/usr/sbin/scutil", "--nc", "status", "RouteLoom"],
            capture_output=True, text=True, timeout=15, check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        raise SystemExit("Unable to query VPN status. No raw diagnostics were printed.")
    print(json.dumps(summarize(output.stdout), indent=2))


if __name__ == "__main__":
    main()
