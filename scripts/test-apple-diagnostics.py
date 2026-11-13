import importlib.util
import json
import pathlib
import plistlib
import unittest

spec = importlib.util.spec_from_file_location("diagnose_apple", pathlib.Path(__file__).with_name("diagnose-apple.py"))
diagnostics = importlib.util.module_from_spec(spec)
spec.loader.exec_module(diagnostics)


class DiagnosticsTest(unittest.TestCase):
    def test_decodes_error_without_disclosing_options_or_error_user_info(self):
        secret = "FAKE-PRIVATE-PROFILE-CREDENTIAL"
        archive = {
            "$version": 100000, "$archiver": "NSKeyedArchiver",
            "$top": {"root": plistlib.UID(1)},
            "$objects": ["$null", {"NSCode": 0, "NSDomain": plistlib.UID(2), "NSUserInfo": secret}, "PacketTunnel.TunnelError"],
        }
        raw = "Disconnected\nExtended Status <dictionary> {\n  ConnectCount : 112\n  ConnectedCount : 0\n  LastDisconnectError : <data> 0x" + plistlib.dumps(archive, fmt=plistlib.FMT_BINARY).hex() + "\n  StartMessage : { profile : " + secret + " }\n}\n"
        result = diagnostics.summarize(raw)
        self.assertEqual(result, {"status": "Disconnected", "ConnectCount": 112, "ConnectedCount": 0, "last_error": {"domain": "PacketTunnel.TunnelError", "code": 0}})
        self.assertNotIn(secret, json.dumps(result))

    def test_unknown_output_is_not_echoed(self):
        self.assertEqual(diagnostics.summarize("PRIVATE-OUTPUT\nprofile : secret"), {"status": "Unavailable"})
        result = diagnostics.summarize("Disconnected\nLastDisconnectError : <data> 0x1234\n")
        self.assertEqual(result["last_error"], {"decode": "unavailable"})


if __name__ == "__main__":
    unittest.main()
